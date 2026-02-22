use super::*;

pub fn compile_struct_methods(codegen: &mut CodeGen, struct_decl: &StructDecl) -> Result<()> {
    for member in &struct_decl.members {
        match member {
            StructMember::Method(method) => compile_struct_method(codegen, struct_decl, method)?,
            StructMember::Constructor(ctor) => {
                compile_struct_constructor(codegen, struct_decl, ctor)?
            }
            StructMember::Default(default_decl) => {
                compile_struct_default(codegen, struct_decl, default_decl)?
            }
            StructMember::Operator(op) => compile_struct_operator(codegen, struct_decl, op)?,
            StructMember::Conversion(conv) => {
                compile_struct_conversion(codegen, struct_decl, conv)?
            }
            StructMember::Field(_) => {}
        }
    }

    let owner = codegen.local_struct_fqcn(&struct_decl.name);
    let synthesized = codegen
        .synthesized_conversions
        .iter()
        .filter_map(|(func_name, conv)| {
            if conv.owner == owner {
                Some((
                    func_name.clone(),
                    conv.target_type.clone(),
                    conv.body.clone(),
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for (_func_name, target_type, body) in synthesized {
        let conv = ConversionDecl {
            visibility: Visibility::Public,
            target_type,
            body,
            span: Span { start: 0, end: 0 },
        };
        compile_struct_conversion(codegen, struct_decl, &conv)?;
    }

    Ok(())
}

fn compile_struct_method(
    codegen: &mut CodeGen,
    struct_decl: &StructDecl,
    method: &MethodDecl,
) -> Result<()> {
    let owner = struct_symbol_owner(codegen, struct_decl);
    let func_name = format!("{}__{}", owner, method.name);
    let func_id = *codegen
        .functions
        .get(&func_name)
        .ok_or_else(|| anyhow!("Method {} not declared", func_name))?;

    let mut ctx = codegen.module.make_context();

    if let Some(ref user_ret) = method.return_type {
        if method.throws {
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

    ctx.func.signature.params.push(AbiParam::new(types::I64));

    for param in &method.params {
        if let MethodParam::Regular(p) = param {
            if let Some(cl_type) = codegen.convert_type(&p.type_expr) {
                ctx.func.signature.params.push(AbiParam::new(cl_type));
            }
        }
    }

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);

    let mut func_ctx = FunctionContext::new();
    func_ctx.is_error_result = method.throws;
    func_ctx.expected_return_type = method.return_type.as_ref().map(|t| {
        crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(codegen, t)
    });

    let params = builder.block_params(entry_block).to_vec();
    let mut param_idx = 0;

    let self_var = func_ctx.create_variable(&mut builder, "__self__", types::I64);
    builder.def_var(self_var, params[param_idx]);
    let struct_fqcn = codegen.local_struct_fqcn(&struct_decl.name);
    func_ctx.set_var_type("__self__", VarType::Struct(struct_fqcn.clone()));
    let self_alias = func_ctx.create_variable(&mut builder, "self", types::I64);
    builder.def_var(self_alias, params[param_idx]);
    func_ctx.set_var_type("self", VarType::Struct(struct_fqcn.clone()));
    func_ctx.current_struct = Some(struct_fqcn.clone());
    param_idx += 1;

    for param in &method.params {
        if let MethodParam::Regular(p) = param {
            if param_idx >= params.len() {
                return Err(anyhow!("Parameter index out of bounds in method {}: AST has more params than IR signature", method.name));
            }
            let ty = codegen.convert_type(&p.type_expr).unwrap_or(types::I64);
            let var = func_ctx.create_variable(&mut builder, &p.name, ty);
            builder.def_var(var, params[param_idx]);
            func_ctx.set_var_type(
                &p.name,
                crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                    codegen,
                    &p.type_expr,
                ),
            );
            param_idx += 1;
        }
    }

    if let Some(type_info) = codegen.type_registry.get(&struct_fqcn) {
        for field in &type_info.fields {
            func_ctx.register_struct_field(&field.name, &struct_fqcn, field.offset);
        }
    }

    builder.seal_block(entry_block);

    let mut last_value = None;
    let mut has_returned = false;
    for stmt in &method.body.statements {
        if let Statement::Return(_) | Statement::Throw(..) = stmt {
            has_returned = true;
        }
        last_value = statement::compile_statement(codegen, &mut builder, &mut func_ctx, stmt)?;

        if has_returned {
            break;
        }
    }

    if !has_returned {
        if func_ctx.is_error_result {
            let zero = builder.ins().iconst(types::I64, 0);
            let val = last_value.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[zero, val]);
        } else if method.return_type.is_some() {
            let val = last_value.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[val]);
        } else if let Some(val) = last_value {
            builder.ins().return_(&[val]);
        } else {
            builder.ins().return_(&[]);
        }
    }

    builder.finalize();

    codegen
        .module
        .define_function(func_id, &mut ctx)
        .map_err(|e| anyhow!("Failed to define method {}: {}", func_name, e))?;

    codegen.module.clear_context(&mut ctx);

    Ok(())
}

fn compile_struct_constructor(
    codegen: &mut CodeGen,
    struct_decl: &StructDecl,
    ctor: &ConstructorDecl,
) -> Result<()> {
    let owner = struct_symbol_owner(codegen, struct_decl);
    let func_name = format!("{}__new", owner);
    let func_id = *codegen
        .functions
        .get(&func_name)
        .ok_or_else(|| anyhow!("Constructor {} not declared", func_name))?;
    let mut ctx = codegen.module.make_context();
    if ctor.throws {
        ctx.func.signature.returns.push(AbiParam::new(types::I64));
    }
    ctx.func.signature.returns.push(AbiParam::new(types::I64));
    for p in &ctor.params {
        if let Some(cl_type) = codegen.convert_type(&p.type_expr) {
            ctx.func.signature.params.push(AbiParam::new(cl_type));
        }
    }
    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);
    let mut func_ctx = FunctionContext::new();
    func_ctx.is_error_result = ctor.throws;
    func_ctx.expected_return_type = Some(VarType::Struct(
        codegen.local_struct_fqcn(&struct_decl.name),
    ));
    let params = builder.block_params(entry_block).to_vec();
    for (i, p) in ctor.params.iter().enumerate() {
        if i >= params.len() {
            break;
        }
        let ty = codegen.convert_type(&p.type_expr).unwrap_or(types::I64);
        let var = func_ctx.create_variable(&mut builder, &p.name, ty);
        builder.def_var(var, params[i]);
        func_ctx.set_var_type(
            &p.name,
            crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                codegen,
                &p.type_expr,
            ),
        );
    }
    builder.seal_block(entry_block);
    let mut last_value = None;
    let mut has_returned = false;
    for stmt in &ctor.body.statements {
        if let Statement::Return(_) | Statement::Throw(..) = stmt {
            has_returned = true;
        }
        last_value = statement::compile_statement(codegen, &mut builder, &mut func_ctx, stmt)?;
        if has_returned {
            break;
        }
    }
    if !has_returned {
        if ctor.throws {
            let zero = builder.ins().iconst(types::I64, 0);
            let val = last_value.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[zero, val]);
        } else {
            let val = last_value.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[val]);
        }
    }
    builder.finalize();
    codegen
        .module
        .define_function(func_id, &mut ctx)
        .map_err(|e| anyhow!("Failed to define constructor {}: {}", func_name, e))?;
    codegen.module.clear_context(&mut ctx);
    Ok(())
}

fn compile_struct_default(
    codegen: &mut CodeGen,
    struct_decl: &StructDecl,
    default_decl: &DefaultDecl,
) -> Result<()> {
    let owner = struct_symbol_owner(codegen, struct_decl);
    let func_name = format!("{}__default", owner);
    let func_id = *codegen
        .functions
        .get(&func_name)
        .ok_or_else(|| anyhow!("Default {} not declared", func_name))?;

    let mut ctx = codegen.module.make_context();
    ctx.func.signature.returns.push(AbiParam::new(types::I64));

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);

    let mut func_ctx = FunctionContext::new();
    func_ctx.expected_return_type = Some(VarType::Struct(
        codegen.local_struct_fqcn(&struct_decl.name),
    ));

    builder.seal_block(entry_block);

    let mut last_value = None;
    let mut has_returned = false;
    for stmt in &default_decl.body.statements {
        if let Statement::Return(_) = stmt {
            has_returned = true;
        }
        last_value = statement::compile_statement(codegen, &mut builder, &mut func_ctx, stmt)?;
        if has_returned {
            break;
        }
    }

    if !has_returned {
        let fallback = crate::codegen::expression::defaults::compile_default_value_fieldwise(
            codegen,
            &mut builder,
            &mut func_ctx,
            &VarType::Struct(codegen.local_struct_fqcn(&struct_decl.name)),
        )?;
        let val = last_value.unwrap_or(fallback);
        builder.ins().return_(&[val]);
    }

    builder.finalize();
    codegen
        .module
        .define_function(func_id, &mut ctx)
        .map_err(|e| anyhow!("Failed to define default {}: {}", func_name, e))?;
    codegen.module.clear_context(&mut ctx);
    Ok(())
}

fn compile_struct_operator(
    codegen: &mut CodeGen,
    struct_decl: &StructDecl,
    op: &OperatorDecl,
) -> Result<()> {
    let owner = struct_symbol_owner(codegen, struct_decl);
    let mangled = mangle_operator(&op.op_symbol);
    let func_name = format!("{}__op_{}", owner, mangled);
    let func_id = *codegen
        .functions
        .get(&func_name)
        .ok_or_else(|| anyhow!("Operator {} not declared", func_name))?;
    let mut ctx = codegen.module.make_context();
    if let Some(ref user_ret) = op.return_type {
        if op.throws {
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
    ctx.func.signature.params.push(AbiParam::new(types::I64));
    for param in &op.params {
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
    func_ctx.is_error_result = op.throws;
    func_ctx.expected_return_type = op.return_type.as_ref().map(|t| {
        crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(codegen, t)
    });
    let params = builder.block_params(entry_block).to_vec();
    let mut param_idx = 0;
    let self_val = params[param_idx];
    let self_var = func_ctx.create_variable(&mut builder, "__self__", types::I64);
    builder.def_var(self_var, self_val);
    let struct_fqcn = codegen.local_struct_fqcn(&struct_decl.name);
    func_ctx.set_var_type("__self__", VarType::Struct(struct_fqcn.clone()));
    let self_alias = func_ctx.create_variable(&mut builder, "self", types::I64);
    builder.def_var(self_alias, self_val);
    func_ctx.set_var_type("self", VarType::Struct(struct_fqcn.clone()));
    func_ctx.current_struct = Some(struct_fqcn.clone());
    param_idx += 1;
    for param in &op.params {
        if param_idx >= params.len() {
            break;
        }
        let ty = codegen.convert_type(&param.type_expr).unwrap_or(types::I64);
        let var = func_ctx.create_variable(&mut builder, &param.name, ty);
        builder.def_var(var, params[param_idx]);
        func_ctx.set_var_type(
            &param.name,
            crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                codegen,
                &param.type_expr,
            ),
        );
        param_idx += 1;
    }
    if let Some(type_info) = codegen.type_registry.get(&struct_fqcn) {
        for field in &type_info.fields {
            func_ctx.register_struct_field(&field.name, &struct_fqcn, field.offset);
        }
    }
    builder.seal_block(entry_block);
    let mut last_value = None;
    let mut has_returned = false;
    for stmt in &op.body.statements {
        if let Statement::Return(_) | Statement::Throw(..) = stmt {
            has_returned = true;
        }
        last_value = statement::compile_statement(codegen, &mut builder, &mut func_ctx, stmt)?;
        if has_returned {
            break;
        }
    }
    if !has_returned {
        if op.throws {
            let zero = builder.ins().iconst(types::I64, 0);
            let val = last_value.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[zero, val]);
        } else if let Some(val) = last_value {
            builder.ins().return_(&[val]);
        } else {
            builder.ins().return_(&[]);
        }
    }
    builder.finalize();
    codegen
        .module
        .define_function(func_id, &mut ctx)
        .map_err(|e| anyhow!("Failed to define operator {}: {}", func_name, e))?;
    codegen.module.clear_context(&mut ctx);
    Ok(())
}

fn compile_struct_conversion(
    codegen: &mut CodeGen,
    struct_decl: &StructDecl,
    conv: &ConversionDecl,
) -> Result<()> {
    let owner = struct_symbol_owner(codegen, struct_decl);
    let target_name = match &conv.target_type {
        TypeExpr::Base(b) => codegen.resolve_struct_type_name(&b.name),
        _ => return Ok(()),
    };
    let func_name = format!("{}__to_{}", owner, target_name);
    let func_id = *codegen
        .functions
        .get(&func_name)
        .ok_or_else(|| anyhow!("Conversion {} not declared", func_name))?;
    let mut ctx = codegen.module.make_context();
    if let Some(cl_type) = codegen.convert_type(&conv.target_type) {
        ctx.func.signature.returns.push(AbiParam::new(cl_type));
    }
    ctx.func.signature.params.push(AbiParam::new(types::I64));
    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);
    let mut func_ctx = FunctionContext::new();
    func_ctx.expected_return_type = Some(
        crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
            codegen,
            &conv.target_type,
        ),
    );
    let params = builder.block_params(entry_block).to_vec();
    let self_var = func_ctx.create_variable(&mut builder, "__self__", types::I64);
    builder.def_var(self_var, params[0]);
    let struct_fqcn = codegen.local_struct_fqcn(&struct_decl.name);
    func_ctx.set_var_type("__self__", VarType::Struct(struct_fqcn.clone()));
    let self_alias = func_ctx.create_variable(&mut builder, "self", types::I64);
    builder.def_var(self_alias, params[0]);
    func_ctx.set_var_type("self", VarType::Struct(struct_fqcn.clone()));
    if let Some(type_info) = codegen.type_registry.get(&struct_fqcn) {
        for field in &type_info.fields {
            func_ctx.register_struct_field(&field.name, &struct_fqcn, field.offset);
        }
    }
    builder.seal_block(entry_block);
    let mut last_value = None;
    let mut has_returned = false;
    for stmt in &conv.body.statements {
        if let Statement::Return(_) = stmt {
            has_returned = true;
        }
        last_value = statement::compile_statement(codegen, &mut builder, &mut func_ctx, stmt)?;
        if has_returned {
            break;
        }
    }
    if !has_returned {
        if let Some(val) = last_value {
            builder.ins().return_(&[val]);
        } else {
            builder.ins().return_(&[]);
        }
    }
    builder.finalize();
    codegen
        .module
        .define_function(func_id, &mut ctx)
        .map_err(|e| anyhow!("Failed to define conversion {}: {}", func_name, e))?;
    codegen.module.clear_context(&mut ctx);
    Ok(())
}
