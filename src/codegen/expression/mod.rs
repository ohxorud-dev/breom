use anyhow::{anyhow, Result};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    types, AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use std::collections::{HashMap, HashSet};

use crate::ast::{expressions::*, statements::*, types::*};
use crate::codegen::context::FunctionContext;
use crate::codegen::types::{DefineValue, VarType};
use crate::codegen::{runtime, statement, CodeGen};

use self::calls::*;
use self::collections::*;
use self::constructors::*;
use self::defaults::*;
use self::lambda::*;
use self::literals::*;
use self::postfix::*;
use self::typing::*;

pub mod calls;
pub mod collections;
pub mod const_eval;
pub mod constructors;
pub mod defaults;
pub mod lambda;
pub mod literals;
pub mod postfix;
pub mod typing;

pub fn compile_expression(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    expr: &Expression,
) -> Result<Value> {
    match expr {
        Expression::Literal(lit) => compile_literal(codegen, builder, ctx, lit),

        Expression::Identifier(name, _) => {
            if let Some(define_value) = codegen.defines.get(name).cloned() {
                match define_value {
                    DefineValue::Int(val) => Ok(builder.ins().iconst(types::I64, val)),
                    DefineValue::Float(val) => Ok(builder.ins().f64const(val)),
                    DefineValue::Bool(val) => {
                        Ok(builder.ins().iconst(types::I64, if val { 1 } else { 0 }))
                    }
                    DefineValue::String(s) => compile_string_literal(codegen, builder, &s),
                    DefineValue::Error(msg) => {
                        let mangled = codegen.mangle_name(name);
                        if let Some(&data_id) = codegen.define_error_globals.get(&mangled) {
                            let data_ref =
                                codegen.module.declare_data_in_func(data_id, builder.func);
                            let addr = builder.ins().symbol_value(types::I64, data_ref);
                            Ok(builder.ins().load(types::I64, MemFlags::new(), addr, 0))
                        } else {
                            let msg_val = compile_string_literal(codegen, builder, &msg)?;
                            runtime::call_runtime(codegen, builder, "breom_error_new", &[msg_val])
                        }
                    }
                }
            } else if ctx.variables.contains_key(name) {
                ctx.get_variable(builder, name)
            } else if let Some((struct_name, offset)) = ctx.get_struct_field(name).cloned() {
                let self_ptr = ctx.get_variable(builder, "__self__")?;
                let field_ptr = builder.ins().iadd_imm(self_ptr, offset as i64);
                let field_ty = codegen
                    .type_registry
                    .get(&codegen.resolve_struct_type_name(&struct_name))
                    .and_then(|type_info| type_info.get_field(name))
                    .map(|field| {
                        clif_type_from_var_type(&var_type_from_type_name_with_codegen(
                            codegen,
                            &field.type_name,
                        ))
                    })
                    .unwrap_or(types::I64);
                let value = builder.ins().load(field_ty, MemFlags::new(), field_ptr, 0);
                Ok(value)
            } else {
                let mangled = codegen.mangle_name(name);
                if let Some(&data_id) = codegen.global_vars.get(&mangled) {
                    let data_ref = codegen.module.declare_data_in_func(data_id, builder.func);
                    let addr = builder.ins().symbol_value(types::I64, data_ref);
                    let value = builder.ins().load(types::I64, MemFlags::new(), addr, 0);
                    Ok(value)
                } else {
                    ctx.get_variable(builder, name)
                }
            }
        }

        Expression::Binary(bin_expr) => {
            let lhs_type = infer_expr_type(codegen, ctx, &bin_expr.left);
            let struct_name = match &lhs_type {
                VarType::Struct(name) => Some(name.clone()),
                VarType::Error => Some("Error".to_string()),
                _ => None,
            };
            if let Some(struct_name) = struct_name {
                let resolved_struct_name = codegen.resolve_struct_type_name(&struct_name);
                let op_str = bin_expr.op.as_str();
                let func_name_opt = codegen
                    .struct_operators
                    .get(&(resolved_struct_name, op_str.to_string()))
                    .cloned();
                if let Some(func_name) = func_name_opt {
                    let lhs = compile_expression(codegen, builder, ctx, &bin_expr.left)?;
                    let rhs = compile_expression(codegen, builder, ctx, &bin_expr.right)?;
                    let arg_vals = vec![lhs, rhs];
                    let func_id = *codegen
                        .functions
                        .get(&func_name)
                        .ok_or_else(|| anyhow!("Operator {} not found", func_name))?;
                    let func_ref = codegen.module.declare_func_in_func(func_id, builder.func);
                    let call = builder.ins().call(func_ref, &arg_vals);
                    let results = builder.inst_results(call);
                    return Ok(if results.is_empty() {
                        builder.ins().iconst(types::I64, 0)
                    } else {
                        results[0]
                    });
                }
            }
            let lhs = compile_expression(codegen, builder, ctx, &bin_expr.left)?;
            let rhs = compile_expression(codegen, builder, ctx, &bin_expr.right)?;
            let lhs_var_type = infer_expr_type(codegen, ctx, &bin_expr.left);
            let rhs_var_type = infer_expr_type(codegen, ctx, &bin_expr.right);
            let is_string_add =
                matches!(lhs_var_type, VarType::String) && matches!(rhs_var_type, VarType::String);
            let is_string_cmp =
                matches!(lhs_var_type, VarType::String) && matches!(rhs_var_type, VarType::String);

            let lhs_ty = builder.func.dfg.value_type(lhs);
            let is_float = lhs_ty == types::F64;

            let result = match bin_expr.op {
                BinaryOp::Add => {
                    if is_string_add {
                        runtime::call_runtime(codegen, builder, "breom_string_concat", &[lhs, rhs])?
                    } else if is_float {
                        builder.ins().fadd(lhs, rhs)
                    } else {
                        builder.ins().iadd(lhs, rhs)
                    }
                }
                BinaryOp::Sub => {
                    if is_float {
                        builder.ins().fsub(lhs, rhs)
                    } else {
                        builder.ins().isub(lhs, rhs)
                    }
                }
                BinaryOp::Mul => {
                    if is_float {
                        builder.ins().fmul(lhs, rhs)
                    } else {
                        builder.ins().imul(lhs, rhs)
                    }
                }
                BinaryOp::Div => {
                    if is_float {
                        builder.ins().fdiv(lhs, rhs)
                    } else {
                        builder.ins().sdiv(lhs, rhs)
                    }
                }
                BinaryOp::Mod => builder.ins().srem(lhs, rhs),

                BinaryOp::BitAnd => builder.ins().band(lhs, rhs),
                BinaryOp::BitOr => builder.ins().bor(lhs, rhs),
                BinaryOp::BitXor => builder.ins().bxor(lhs, rhs),
                BinaryOp::Shl => builder.ins().ishl(lhs, rhs),
                BinaryOp::Shr => builder.ins().sshr(lhs, rhs),

                BinaryOp::Eq => {
                    if is_string_cmp {
                        runtime::call_runtime(codegen, builder, "breom_string_eq", &[lhs, rhs])?
                    } else {
                        let cmp = if is_float {
                            builder.ins().fcmp(FloatCC::Equal, lhs, rhs)
                        } else {
                            builder.ins().icmp(IntCC::Equal, lhs, rhs)
                        };
                        builder.ins().uextend(types::I64, cmp)
                    }
                }
                BinaryOp::Ne => {
                    if is_string_cmp {
                        let eq = runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_string_eq",
                            &[lhs, rhs],
                        )?;
                        let zero = builder.ins().iconst(types::I64, 0);
                        let cmp = builder.ins().icmp(IntCC::Equal, eq, zero);
                        builder.ins().uextend(types::I64, cmp)
                    } else {
                        let cmp = if is_float {
                            builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs)
                        } else {
                            builder.ins().icmp(IntCC::NotEqual, lhs, rhs)
                        };
                        builder.ins().uextend(types::I64, cmp)
                    }
                }
                BinaryOp::Lt => {
                    let cmp = if is_float {
                        builder.ins().fcmp(FloatCC::LessThan, lhs, rhs)
                    } else {
                        builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs)
                    };
                    builder.ins().uextend(types::I64, cmp)
                }
                BinaryOp::Le => {
                    let cmp = if is_float {
                        builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs)
                    } else {
                        builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs)
                    };
                    builder.ins().uextend(types::I64, cmp)
                }
                BinaryOp::Gt => {
                    let cmp = if is_float {
                        builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs)
                    } else {
                        builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs)
                    };
                    builder.ins().uextend(types::I64, cmp)
                }
                BinaryOp::Ge => {
                    let cmp = if is_float {
                        builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs)
                    } else {
                        builder
                            .ins()
                            .icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs)
                    };
                    builder.ins().uextend(types::I64, cmp)
                }

                BinaryOp::And => builder.ins().band(lhs, rhs),
                BinaryOp::Or => builder.ins().bor(lhs, rhs),
            };

            Ok(result)
        }

        Expression::Unary(unary_expr) => {
            let operand = compile_expression(codegen, builder, ctx, &unary_expr.operand)?;

            let ty = builder.func.dfg.value_type(operand);
            let is_float = ty == types::F64;

            let result = match unary_expr.op {
                UnaryOp::Neg => {
                    if is_float {
                        builder.ins().fneg(operand)
                    } else {
                        builder.ins().ineg(operand)
                    }
                }
                UnaryOp::Not => {
                    let zero = if is_float {
                        builder.ins().f64const(0.0)
                    } else {
                        builder.ins().iconst(types::I64, 0)
                    };
                    let cmp = if is_float {
                        builder.ins().fcmp(FloatCC::Equal, operand, zero)
                    } else {
                        builder.ins().icmp(IntCC::Equal, operand, zero)
                    };
                    builder.ins().uextend(types::I64, cmp)
                }
                UnaryOp::BitNot => builder.ins().bnot(operand),
            };

            Ok(result)
        }

        Expression::Grouped(inner, _) => compile_expression(codegen, builder, ctx, inner),

        Expression::Postfix(postfix) => compile_postfix(codegen, builder, ctx, postfix),

        Expression::Ternary(ternary) => {
            let result_var = builder.declare_var(types::I64);

            let cond_val = compile_expression(codegen, builder, ctx, &ternary.condition)?;

            let then_block = builder.create_block();
            let else_block = builder.create_block();
            let merge_block = builder.create_block();

            let zero = builder.ins().iconst(types::I64, 0);
            let cond = builder.ins().icmp(IntCC::NotEqual, cond_val, zero);
            builder.ins().brif(cond, then_block, &[], else_block, &[]);

            builder.switch_to_block(then_block);
            builder.seal_block(then_block);
            let then_val = compile_expression(codegen, builder, ctx, &ternary.then_expr)?;
            builder.def_var(result_var, then_val);
            builder.ins().jump(merge_block, &[]);

            builder.switch_to_block(else_block);
            builder.seal_block(else_block);
            let else_val = compile_expression(codegen, builder, ctx, &ternary.else_expr)?;
            builder.def_var(result_var, else_val);
            builder.ins().jump(merge_block, &[]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let result = builder.use_var(result_var);

            Ok(result)
        }

        Expression::StructLiteral(struct_lit) => {
            compile_struct_literal(codegen, builder, ctx, struct_lit)
        }

        Expression::Collection(coll) => compile_collection(codegen, builder, ctx, coll),

        Expression::Lambda(lambda) => compile_lambda(codegen, builder, ctx, lambda),

        Expression::TupleLiteral(tuple) => compile_tuple(codegen, builder, ctx, tuple),

        Expression::ChannelReceive(expr, _) => {
            let chan_val = compile_expression(codegen, builder, ctx, expr)?;
            runtime::call_runtime(codegen, builder, "breom_chan_recv", &[chan_val])
        }
        Expression::ChannelNew(channel_new) => {
            compile_channel_new(codegen, builder, ctx, channel_new)
        }
        Expression::New(new_expr) => compile_new(codegen, builder, ctx, new_expr),
    }
}

pub fn compile_expression_with_type_hint(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    expr: &Expression,
    expected_type: Option<&VarType>,
) -> Result<Value> {
    if let Some(VarType::StaticArray(elem_type, declared_len)) = expected_type {
        if let Expression::Collection(coll) = expr {
            return compile_collection_for_static_array(
                codegen,
                builder,
                ctx,
                coll,
                elem_type,
                *declared_len,
            );
        }
    }
    compile_expression(codegen, builder, ctx, expr)
}
