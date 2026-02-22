use anyhow::{anyhow, Result};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};

use crate::ast::{common::*, declarations::*, statements::*, types::*};
use crate::codegen::context::FunctionContext;
use crate::codegen::statement;
use crate::codegen::{wrap_return_type, CodeGen};

pub fn declare_function(codegen: &mut CodeGen, func: &FunctionDecl) -> Result<FuncId> {
    if func.name == "main" {
        if !func.params.is_empty() {
            return Err(anyhow!("'main' function must not have arguments"));
        }

        match &func.return_type {
            Some(TypeExpr::Base(base)) if base.name == "Int" => {
                codegen.main_returns_int = true;
                codegen.main_throws = func.throws;
            }
            Some(TypeExpr::Base(base)) if base.name == "Void" => {
                codegen.main_returns_int = false;
                codegen.main_throws = false;
            }
            None => {
                codegen.main_returns_int = false;
                codegen.main_throws = false;
            }
            _ => return Err(anyhow!("'main' function must return Int or Void")),
        }
    }

    let mangled_name = codegen.mangle_name(&func.name);
    let mut sig = codegen.module.make_signature();
    let value_return_type = func
        .return_type
        .as_ref()
        .map(|ret| {
            crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                codegen, ret,
            )
        })
        .unwrap_or(crate::codegen::types::VarType::Unknown);
    if let Some(ref user_ret) = func.return_type {
        if func.throws {
            let actual_ret = wrap_return_type(user_ret);
            codegen
                .function_return_types
                .insert(mangled_name.clone(), actual_ret.clone());
            sig.returns.push(AbiParam::new(types::I64));
            if let TypeExpr::Tuple(tt) = &actual_ret {
                if tt.element_types.len() >= 2 {
                    if let Some(cl) = codegen.convert_type(&tt.element_types[1].type_expr) {
                        sig.returns.push(AbiParam::new(cl));
                    } else {
                        sig.returns.push(AbiParam::new(types::I64));
                    }
                }
            }
        } else if let Some(cl_type) = codegen.convert_type(user_ret) {
            sig.returns.push(AbiParam::new(cl_type));
        }
    }

    let mut param_types = Vec::new();
    let mut param_type_exprs = Vec::new();
    for param in &func.params {
        if let Some(cl_type) = codegen.convert_type(&param.type_expr) {
            sig.params.push(AbiParam::new(cl_type));
        }
        param_types.push(
            crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                codegen,
                &param.type_expr,
            ),
        );
        param_type_exprs.push(param.type_expr.clone());
    }

    let mangled_name = codegen.mangle_name(&func.name);
    let func_id = codegen
        .module
        .declare_function(&mangled_name, Linkage::Export, &sig)
        .map_err(|e| anyhow!("Failed to declare function {}: {}", mangled_name, e))?;

    codegen.functions.insert(mangled_name.clone(), func_id);
    codegen
        .function_value_types
        .insert(mangled_name.clone(), value_return_type);
    codegen
        .function_param_types
        .insert(mangled_name.clone(), param_types);
    codegen
        .function_param_type_exprs
        .insert(mangled_name.clone(), param_type_exprs);
    if !func.generic_params.is_empty() {
        codegen
            .function_generic_params
            .insert(mangled_name.clone(), func.generic_params.clone());
    }
    let is_public = matches!(func.visibility, Visibility::Public);
    codegen.function_visibility.insert(mangled_name, is_public);
    if func.name == "main" && codegen.current_package == codegen.entry_package {
        codegen.functions.insert("main".to_string(), func_id);
    }
    Ok(func_id)
}

pub fn compile_function(codegen: &mut CodeGen, func: &FunctionDecl) -> Result<()> {
    let mangled_name = codegen.mangle_name(&func.name);
    let func_id = *codegen
        .functions
        .get(&mangled_name)
        .or_else(|| {
            if func.name == "main" {
                codegen.functions.get("main")
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("Function {} not declared", mangled_name))?;

    let mut ctx = codegen.module.make_context();

    if let Some(ref user_ret) = func.return_type {
        if func.throws {
            let actual_ret = wrap_return_type(user_ret);
            ctx.func.signature.returns.push(AbiParam::new(types::I64));
            if let TypeExpr::Tuple(tt) = &actual_ret {
                if tt.element_types.len() >= 2 {
                    if let Some(cl) = codegen.convert_type(&tt.element_types[1].type_expr) {
                        ctx.func.signature.returns.push(AbiParam::new(cl));
                    } else {
                        ctx.func.signature.returns.push(AbiParam::new(types::I64));
                    }
                }
            }
        } else if let Some(cl_type) = codegen.convert_type(user_ret) {
            ctx.func.signature.returns.push(AbiParam::new(cl_type));
        }
    }

    for param in &func.params {
        if let Some(cl_type) = codegen.convert_type(&param.type_expr) {
            ctx.func.signature.params.push(AbiParam::new(cl_type));
        }
    }

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);

    let mut func_ctx = FunctionContext::new();
    func_ctx.is_error_result = func.throws;
    func_ctx.expected_return_type = func
        .return_type
        .as_ref()
        .map(crate::codegen::expression::typing::infer_type_expr_to_var_type);

    let params = builder.block_params(entry_block).to_vec();
    for (i, param) in func.params.iter().enumerate() {
        if i >= params.len() {
            return Err(anyhow!("Parameter index out of bounds in function {}: AST has more params than IR signature", func.name));
        }
        let ty = codegen.convert_type(&param.type_expr).unwrap_or(types::I64);
        let var = func_ctx.create_variable(&mut builder, &param.name, ty);
        builder.def_var(var, params[i]);
        func_ctx.set_var_type(
            &param.name,
            crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                codegen,
                &param.type_expr,
            ),
        );
    }

    builder.seal_block(entry_block);

    let mut last_value = None;
    let mut has_returned = false;
    for stmt in &func.body.statements {
        if let Statement::Return(_) | Statement::Throw(..) = stmt {
            has_returned = true;
        }
        last_value = statement::compile_statement(codegen, &mut builder, &mut func_ctx, stmt)?;

        if has_returned {
            break;
        }
    }

    if !has_returned {
        if func.throws {
            let zero = builder.ins().iconst(types::I64, 0);
            let val = last_value.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[zero, val]);
        } else if func.return_type.is_some() {
            let val = last_value.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[val]);
        } else {
            builder.ins().return_(&[]);
        }
    }

    builder.finalize();

    codegen
        .module
        .define_function(func_id, &mut ctx)
        .map_err(|e| anyhow!("Failed to define function {}: {}", func.name, e))?;

    codegen.module.clear_context(&mut ctx);

    Ok(())
}
