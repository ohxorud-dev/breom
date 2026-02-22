use super::*;

type PromotedPointField = (String, String, u64, u64, String);

const ARRAY_LEN_OFFSET: i32 = 0;
const ARRAY_DATA_OFFSET: i64 = 24;
const ARRAY_INDEX_SHIFT: i64 = 3;

pub(super) fn compile_array_index_ptr(
    builder: &mut FunctionBuilder,
    array_ptr: Value,
    index: Value,
) -> Value {
    let data_ptr = builder.ins().iadd_imm(array_ptr, ARRAY_DATA_OFFSET);
    let byte_offset = builder.ins().ishl_imm(index, ARRAY_INDEX_SHIFT);
    builder.ins().iadd(data_ptr, byte_offset)
}

pub(super) fn split_statement_level_instead(expr: &Expression) -> Option<(Expression, Expression)> {
    let Expression::Postfix(postfix) = expr else {
        return None;
    };

    let idx = postfix
        .ops
        .iter()
        .position(|op| matches!(op, PostfixOp::Instead(_)))?;

    let fallback = match &postfix.ops[idx] {
        PostfixOp::Instead(expr) => (**expr).clone(),
        _ => unreachable!(),
    };

    let prefix = if idx == 0 {
        (*postfix.base).clone()
    } else {
        Expression::Postfix(PostfixExpr {
            base: postfix.base.clone(),
            ops: postfix.ops[..idx].to_vec(),
            span: postfix.span.clone(),
        })
    };

    Some((prefix, fallback))
}

pub(super) fn compile_postfix(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    postfix: &PostfixExpr,
) -> Result<Value> {
    let has_error_handling = postfix.ops.iter().any(|o| {
        matches!(
            o,
            PostfixOp::Catch(_) | PostfixOp::ErrorProp | PostfixOp::Instead(_)
        )
    });
    if has_error_handling {
        ctx.in_result_context = true;
        ctx.result_error = None;
        ctx.result_value = None;
    }

    if let Expression::Identifier(name, _) = postfix.base.as_ref() {
        if name == "Chan" {
            let mut buffer_size = builder.ins().iconst(types::I64, 0);
            let mut i = 0;
            let mut found_new = false;
            while i < postfix.ops.len() {
                match &postfix.ops[i] {
                    PostfixOp::Member(m) if m == "new" => {
                        found_new = true;
                        if i + 1 < postfix.ops.len() {
                            if let PostfixOp::Call(args) = &postfix.ops[i + 1] {
                                if !args.is_empty() {
                                    buffer_size =
                                        compile_expression(codegen, builder, ctx, &args[0])?;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if found_new {
                return runtime::call_runtime(codegen, builder, "breom_chan_new", &[buffer_size]);
            }
        }

        if codegen
            .type_registry
            .get(&codegen.resolve_struct_type_name(name))
            .is_some()
        {
            if let (Some(PostfixOp::Member(m)), Some(PostfixOp::Call(args))) =
                (postfix.ops.first(), postfix.ops.get(1))
            {
                if m == "new" {
                    let resolved_name = codegen.resolve_struct_type_name(name);
                    let func_name = format!("{}__new", resolved_name);
                    if codegen.functions.contains_key(&func_name) {
                        let mut result = compile_call(codegen, builder, ctx, &func_name, args)?;
                        let mut result_type = VarType::Struct(resolved_name);
                        for op in postfix.ops.iter().skip(2) {
                            result = apply_postfix_op(
                                codegen,
                                builder,
                                ctx,
                                result,
                                Some(&result_type),
                                op,
                            )?;
                            result_type = infer_postfix_result_type(codegen, &result_type, op);
                        }
                        return Ok(result);
                    }
                } else if m == "default" {
                    let resolved_name = codegen.resolve_struct_type_name(name);
                    let func_name = format!("{}__default", resolved_name);
                    if codegen.functions.contains_key(&func_name) {
                        let mut result = compile_call(codegen, builder, ctx, &func_name, args)?;
                        let mut result_type = VarType::Struct(resolved_name);
                        for op in postfix.ops.iter().skip(2) {
                            result = apply_postfix_op(
                                codegen,
                                builder,
                                ctx,
                                result,
                                Some(&result_type),
                                op,
                            )?;
                            result_type = infer_postfix_result_type(codegen, &result_type, op);
                        }
                        return Ok(result);
                    }

                    if !args.is_empty() {
                        return Err(anyhow!(
                            "{}.default() does not take arguments",
                            resolved_name
                        ));
                    }

                    let mut result = compile_default_value(
                        codegen,
                        builder,
                        ctx,
                        &VarType::Struct(resolved_name.clone()),
                    )?;
                    let mut result_type = VarType::Struct(resolved_name);
                    for op in postfix.ops.iter().skip(2) {
                        result = apply_postfix_op(
                            codegen,
                            builder,
                            ctx,
                            result,
                            Some(&result_type),
                            op,
                        )?;
                        result_type = infer_postfix_result_type(codegen, &result_type, op);
                    }
                    return Ok(result);
                }
            }
        }
    }

    if let Expression::Identifier(name, _) = postfix.base.as_ref() {
        if let Some(PostfixOp::Call(args)) = postfix.ops.first() {
            let mangled = codegen.mangle_name(name);
            if !ctx.variables.contains_key(name)
                && (name == "println"
                    || name == "print"
                    || name == "len"
                    || name == "string"
                    || name == "error"
                    || name == "sleep"
                    || (codegen.is_test_mode() && (name == "assert" || name == "fail"))
                    || codegen.functions.contains_key(name)
                    || codegen.functions.contains_key(&mangled))
            {
                let mut current = compile_call(codegen, builder, ctx, name, args)?;
                let mut current_type = codegen
                    .function_value_types
                    .get(&mangled)
                    .cloned()
                    .or_else(|| codegen.function_value_types.get(name).cloned())
                    .unwrap_or(VarType::Unknown);

                for op in postfix.ops.iter().skip(1) {
                    current =
                        apply_postfix_op(codegen, builder, ctx, current, Some(&current_type), op)?;
                    current_type = infer_postfix_result_type(codegen, &current_type, op);
                }
                return Ok(current);
            }
        }
    }

    let (mut current, mut current_type, mut i) = 'init: {
        if let Expression::Identifier(name, _) = postfix.base.as_ref() {
            if let Some(struct_name) = ctx.current_struct.clone() {
                let method_name = format!("{}__{}", struct_name, name);

                if codegen.functions.contains_key(&method_name) && !postfix.ops.is_empty() {
                    if let PostfixOp::Call(args) = &postfix.ops[0] {
                        if let Ok(self_val) = ctx
                            .get_variable(builder, "__self__")
                            .or_else(|_| ctx.get_variable(builder, "self"))
                        {
                            let res = compile_struct_method_call(
                                codegen,
                                builder,
                                ctx,
                                Some(&struct_name),
                                self_val,
                                &method_name,
                                args,
                            )?;
                            let ret_ty = codegen
                                .function_value_types
                                .get(&method_name)
                                .cloned()
                                .unwrap_or(VarType::Unknown);
                            break 'init (res, ret_ty, 1);
                        }
                    }
                }

                if !ctx.variables.contains_key(name) {
                    if let Some((_, offset)) = ctx.struct_fields.get(name) {
                        if let Ok(self_val) = ctx
                            .get_variable(builder, "__self__")
                            .or_else(|_| ctx.get_variable(builder, "self"))
                        {
                            let field_ptr = builder.ins().iadd_imm(self_val, *offset as i64);
                            let field_ty = codegen
                                .type_registry
                                .get(&codegen.resolve_struct_type_name(&struct_name))
                                .and_then(|type_info| type_info.get_field(name))
                                .map(|field| {
                                    var_type_from_type_name_with_codegen(codegen, &field.type_name)
                                })
                                .unwrap_or(VarType::Unknown);
                            let clif_ty = clif_type_from_var_type(&field_ty);
                            let val = builder.ins().load(
                                clif_ty,
                                cranelift_codegen::ir::MemFlags::new(),
                                field_ptr,
                                0,
                            );
                            break 'init (val, field_ty, 0);
                        }
                    }
                }
            }

            if !ctx.variables.contains_key(name) && codegen.current_imports.contains_key(name) {
                if let (Some(PostfixOp::Member(method_name)), Some(PostfixOp::Call(args))) =
                    (postfix.ops.first(), postfix.ops.get(1))
                {
                    let full_pkg = codegen.current_imports.get(name).unwrap();
                    let target_func = format!("{}.{}", full_pkg, method_name);
                    let res = compile_call(codegen, builder, ctx, &target_func, args)?;
                    let ret_ty = codegen
                        .function_value_types
                        .get(&target_func)
                        .cloned()
                        .unwrap_or(VarType::Unknown);
                    break 'init (res, ret_ty, 2);
                }
            }
        }

        let val = compile_expression(codegen, builder, ctx, &postfix.base)?;
        let ty = infer_expr_type(codegen, ctx, &postfix.base);
        (val, ty, 0)
    };
    let ops = &postfix.ops;
    while i < ops.len() {
        if i + 1 < ops.len() {
            if let (PostfixOp::Member(method_name), PostfixOp::Call(args)) = (&ops[i], &ops[i + 1])
            {
                let var_type = current_type.clone();

                if let Some(result) = compile_builtin_method(
                    codegen,
                    builder,
                    ctx,
                    current,
                    &var_type,
                    method_name,
                    args,
                )? {
                    current = result;
                    if method_name == "push" {
                        if let Expression::Identifier(var_name, _) = postfix.base.as_ref() {
                            if let VarType::DynamicArray(_) = var_type {
                                ctx.set_variable(builder, var_name, current)?;
                            }
                        }
                        current_type = var_type;
                    } else {
                        current_type =
                            infer_builtin_method_result_type(&var_type, method_name, args.len())
                                .unwrap_or(VarType::Unknown);
                    }
                    i += 2;
                    continue;
                }

                if let VarType::Struct(struct_name) = &var_type {
                    if let Some(func_name) =
                        codegen.try_resolve_struct_method_name(struct_name, method_name)?
                    {
                        current = compile_struct_method_call(
                            codegen,
                            builder,
                            ctx,
                            Some(struct_name),
                            current,
                            &func_name,
                            args,
                        )?;
                        current_type = codegen
                            .function_value_types
                            .get(&func_name)
                            .cloned()
                            .unwrap_or(VarType::Unknown);
                        i += 2;
                        continue;
                    }

                    let resolved_struct_name = codegen.resolve_struct_type_name(struct_name);
                    if let Some((point_offset, func_name, point_field_name, promoted_struct_name)) =
                        resolve_promoted_point_method(codegen, &resolved_struct_name, method_name)?
                    {
                        codegen.check_field_access(&resolved_struct_name, &point_field_name)?;
                        let offset = builder.ins().iconst(types::I64, point_offset as i64);
                        let point_ptr_ptr = builder.ins().iadd(current, offset);
                        let point_self =
                            builder
                                .ins()
                                .load(types::I64, MemFlags::new(), point_ptr_ptr, 0);
                        current = compile_struct_method_call(
                            codegen,
                            builder,
                            ctx,
                            Some(&promoted_struct_name),
                            point_self,
                            &func_name,
                            args,
                        )?;
                        current_type = codegen
                            .function_value_types
                            .get(&func_name)
                            .cloned()
                            .unwrap_or(VarType::Unknown);
                        i += 2;
                        continue;
                    }
                }
            }
        }

        current = apply_postfix_op(codegen, builder, ctx, current, Some(&current_type), &ops[i])?;
        current_type = infer_postfix_result_type(codegen, &current_type, &ops[i]);
        i += 1;
    }

    Ok(current)
}

pub(super) fn apply_postfix_op(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    current: Value,
    current_type: Option<&VarType>,
    op: &PostfixOp,
) -> Result<Value> {
    match op {
        PostfixOp::Call(args) => {
            compile_indirect_call(codegen, builder, ctx, current, current_type, args)
        }
        PostfixOp::Member(field_name) => match current_type {
            Some(VarType::Struct(struct_name)) => {
                let resolved_struct_name = codegen.resolve_struct_type_name(struct_name);
                codegen.ensure_instantiated_struct_type(&resolved_struct_name)?;
                let type_info = codegen
                    .type_registry
                    .get(&resolved_struct_name)
                    .ok_or_else(|| anyhow!("Unknown struct type: {}", resolved_struct_name))?;
                let mut field_offsets = Vec::new();
                let field_type_name: String;
                let mut owner_type_name = resolved_struct_name.clone();
                if let Some(field) = type_info.get_field(field_name) {
                    field_offsets.push(field.offset);
                    field_type_name = field.type_name.clone();
                } else if let Some((
                    owner,
                    promoted_type,
                    point_offset,
                    field_offset,
                    point_field_name,
                )) =
                    resolve_promoted_point_field(codegen, &resolved_struct_name, field_name)?
                {
                    codegen.check_field_access(&resolved_struct_name, &point_field_name)?;
                    field_offsets.push(point_offset);
                    field_offsets.push(field_offset);
                    field_type_name = promoted_type;
                    owner_type_name = owner;
                } else {
                    return Err(anyhow!(
                        "Unknown field: {}.{}",
                        resolved_struct_name,
                        field_name
                    ));
                }
                codegen.check_field_access(&owner_type_name, field_name)?;

                let mut ptr = current;
                if field_offsets.len() == 2 {
                    let first_offset = builder.ins().iconst(types::I64, field_offsets[0] as i64);
                    let point_ptr_ptr = builder.ins().iadd(ptr, first_offset);
                    ptr = builder
                        .ins()
                        .load(types::I64, MemFlags::new(), point_ptr_ptr, 0);
                }
                let offset = builder
                    .ins()
                    .iconst(types::I64, *field_offsets.last().unwrap_or(&0) as i64);
                let field_ptr = builder.ins().iadd(ptr, offset);
                let field_var_type =
                    var_type_from_type_name_with_codegen(codegen, &field_type_name);
                let field_ty = clif_type_from_var_type(&field_var_type);
                let value = builder.ins().load(field_ty, MemFlags::new(), field_ptr, 0);
                if field_var_type.is_heap_type() {
                    runtime::arc_retain(codegen, builder, value)?;
                }
                Ok(value)
            }
            Some(VarType::Unknown) | None => Err(anyhow!(
                "Cannot resolve member '{}' without struct receiver type",
                field_name
            )),
            Some(other) => Err(anyhow!("Member access on non-struct type: {:?}", other)),
        },
        PostfixOp::Index(index_expr) => {
            let index = compile_expression(codegen, builder, ctx, index_expr)?;
            match current_type {
                Some(VarType::DynamicArray(_)) | Some(VarType::StaticArray(_, _)) => {
                    let elem_ptr = compile_array_index_ptr(builder, current, index);
                    Ok(builder.ins().load(types::I64, MemFlags::new(), elem_ptr, 0))
                }
                Some(VarType::Tuple(_)) => {
                    let res = runtime::call_runtime(
                        codegen,
                        builder,
                        "breom_array_get",
                        &[current, index],
                    )?;
                    Ok(res)
                }
                Some(VarType::Map(_, _)) => {
                    let res = runtime::call_runtime(
                        codegen,
                        builder,
                        "breom_map_get",
                        &[current, index],
                    )?;
                    Ok(res)
                }
                Some(VarType::Unknown) | None => {
                    let res = runtime::call_runtime(
                        codegen,
                        builder,
                        "breom_array_get",
                        &[current, index],
                    )?;
                    Ok(res)
                }
                Some(other) => Err(anyhow!("Index access on non-indexable type: {:?}", other)),
            }
        }
        PostfixOp::Cast(target_type) => {
            let target_var_type = infer_type_expr_to_var_type_with_codegen(codegen, target_type);

            if matches!(target_var_type, VarType::String) {
                let src_type = current_type.unwrap_or(&VarType::Unknown);
                match src_type {
                    VarType::Int => {
                        return runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_int_to_string",
                            &[current],
                        );
                    }
                    VarType::Float => {
                        return runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_float_to_string",
                            &[current],
                        );
                    }
                    VarType::String => return Ok(current),
                    _ => {}
                }
            }

            let target_name = if let TypeExpr::Base(b) = target_type {
                codegen.resolve_struct_type_name(&b.name)
            } else {
                return Ok(current);
            };

            let src_type = current_type.unwrap_or(&VarType::Unknown);
            let src_name = match src_type {
                VarType::Struct(name) => Some(name.clone()),
                VarType::Error => Some("Error".to_string()),
                _ => None,
            };

            if let Some(src_name) = src_name {
                let resolved_src_name = codegen.resolve_struct_type_name(&src_name);
                if let Some(func_name) =
                    codegen.try_resolve_struct_conversion_name(&resolved_src_name, &target_name)?
                {
                    return compile_struct_method_call(
                        codegen,
                        builder,
                        ctx,
                        Some(&resolved_src_name),
                        current,
                        &func_name,
                        &[],
                    );
                }
            }

            if matches!(target_var_type, VarType::String) {
                return Err(anyhow!(
                    "Unsupported cast to String from {:?}. Implement StringConvertable with `to String`.",
                    src_type
                ));
            }

            Ok(current)
        }
        PostfixOp::ErrorProp => {
            if ctx.result_error.is_none() {
                return Err(anyhow!(
                    "'?' can only be used on expressions returning Error result"
                ));
            }
            if let Some(err_val) = ctx.result_error {
                let zero = builder.ins().iconst(types::I64, 0);
                let is_err = builder.ins().icmp(IntCC::NotEqual, err_val, zero);

                let ok_block = builder.create_block();
                let prop_block = builder.create_block();

                builder.ins().brif(is_err, prop_block, &[], ok_block, &[]);

                builder.switch_to_block(prop_block);
                builder.seal_block(prop_block);

                runtime::release_all_heap_vars(codegen, builder, ctx)?;
                statement::defer::execute_defers(codegen, builder, ctx)?;
                builder.ins().return_(&[err_val, zero]);

                builder.switch_to_block(ok_block);
                builder.seal_block(ok_block);

                ctx.result_error = None;
            }
            Ok(current)
        }
        PostfixOp::Catch(block) => {
            if let Some(err_val) = ctx.result_error {
                let zero = builder.ins().iconst(types::I64, 0);
                let is_err = builder.ins().icmp(IntCC::NotEqual, err_val, zero);

                let ok_block = builder.create_block();
                let catch_block = builder.create_block();
                let merge_block = builder.create_block();

                let result_var = builder.declare_var(builder.func.dfg.value_type(current));
                builder.def_var(result_var, current);

                builder.ins().brif(is_err, catch_block, &[], ok_block, &[]);

                builder.switch_to_block(catch_block);
                builder.seal_block(catch_block);

                let old_err_var = ctx.variables.get("err").cloned();
                let old_err_type = ctx.var_types.get("err").cloned();

                let err_var = ctx.create_variable(builder, "err", types::I64);
                builder.def_var(err_var, err_val);
                ctx.set_var_type("err", VarType::Error);

                let mut catch_val = current;
                let mut has_instead = false;
                for stmt in &block.statements {
                    if let Statement::Expression(expr_stmt) = stmt {
                        if let Some((prefix, fallback)) = split_statement_level_instead(expr_stmt) {
                            compile_expression(codegen, builder, ctx, &prefix)?;
                            ctx.result_error = Some(err_val);
                            catch_val = compile_expression(codegen, builder, ctx, &fallback)?;
                            has_instead = true;
                            break;
                        }
                    }

                    if let Some(v) = statement::compile_statement(codegen, builder, ctx, stmt)? {
                        catch_val = v;
                    }
                    if matches!(stmt, Statement::Instead(..)) {
                        has_instead = true;
                        break;
                    }
                }

                if !has_instead {
                    return Err(anyhow!("catch block must use `instead <expr>`"));
                }

                if let Some(old) = old_err_var {
                    ctx.variables.insert("err".to_string(), old);
                } else {
                    ctx.variables.remove("err");
                }
                if let Some(old_ty) = old_err_type {
                    ctx.var_types.insert("err".to_string(), old_ty);
                } else {
                    ctx.var_types.remove("err");
                }

                builder.def_var(result_var, catch_val);
                builder.ins().jump(merge_block, &[]);

                builder.switch_to_block(ok_block);
                builder.seal_block(ok_block);
                builder.ins().jump(merge_block, &[]);

                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);

                let final_val = builder.use_var(result_var);
                ctx.result_error = None;
                return Ok(final_val);
            }
            Ok(current)
        }
        PostfixOp::Instead(fallback_expr) => {
            if let Some(err_val) = ctx.result_error {
                let zero = builder.ins().iconst(types::I64, 0);
                let is_err = builder.ins().icmp(IntCC::NotEqual, err_val, zero);

                let ok_block = builder.create_block();
                let fallback_block = builder.create_block();
                let merge_block = builder.create_block();

                let result_var = builder.declare_var(builder.func.dfg.value_type(current));
                builder.def_var(result_var, current);

                builder
                    .ins()
                    .brif(is_err, fallback_block, &[], ok_block, &[]);

                builder.switch_to_block(fallback_block);
                builder.seal_block(fallback_block);
                let fallback_val = compile_expression(codegen, builder, ctx, fallback_expr)?;
                builder.def_var(result_var, fallback_val);
                builder.ins().jump(merge_block, &[]);

                builder.switch_to_block(ok_block);
                builder.seal_block(ok_block);
                builder.ins().jump(merge_block, &[]);

                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);

                let final_val = builder.use_var(result_var);
                ctx.result_error = None;
                return Ok(final_val);
            }
            Ok(current)
        }
        PostfixOp::ChannelSend(expr_inner) => {
            let val = compile_expression(codegen, builder, ctx, expr_inner)?;
            runtime::call_runtime(codegen, builder, "breom_chan_send", &[current, val])?;
            Ok(current)
        }
    }
}

pub(super) fn infer_postfix_result_type(
    codegen: &CodeGen,
    current_type: &VarType,
    op: &PostfixOp,
) -> VarType {
    match op {
        PostfixOp::Member(field_name) => {
            if let VarType::Struct(struct_name) = current_type {
                let resolved = codegen.resolve_struct_type_name(struct_name);
                if let Some(type_info) = codegen.type_registry.get(&resolved) {
                    if let Some(field) = type_info.get_field(field_name) {
                        return var_type_from_type_name_with_codegen(codegen, &field.type_name);
                    }
                }

                if let Ok(Some((_owner, promoted_type, ..))) =
                    resolve_promoted_point_field(codegen, &resolved, field_name)
                {
                    return var_type_from_type_name_with_codegen(codegen, &promoted_type);
                }
            }
            VarType::Unknown
        }
        PostfixOp::Index(index_expr) => match current_type {
            VarType::DynamicArray(inner) | VarType::StaticArray(inner, _) => (**inner).clone(),
            VarType::Tuple(elements) => {
                if let Some(idx) = const_non_negative_int(index_expr) {
                    elements.get(idx).cloned().unwrap_or(VarType::Unknown)
                } else {
                    VarType::Unknown
                }
            }
            VarType::Map(_, value_ty) => (**value_ty).clone(),
            _ => VarType::Unknown,
        },
        PostfixOp::Cast(target_type) => {
            infer_type_expr_to_var_type_with_codegen(codegen, target_type)
        }
        PostfixOp::Call(_) => match current_type {
            VarType::Lambda { return_type, .. } => (**return_type).clone(),
            _ => VarType::Unknown,
        },
        PostfixOp::ErrorProp | PostfixOp::Catch(_) | PostfixOp::Instead(_) => current_type.clone(),
        _ => VarType::Unknown,
    }
}

pub(super) fn resolve_promoted_point_field(
    codegen: &CodeGen,
    struct_name: &str,
    field_name: &str,
) -> Result<Option<PromotedPointField>> {
    let Some(point_fields) = codegen.struct_point_fields.get(struct_name) else {
        return Ok(None);
    };
    let Some(owner_info) = codegen.type_registry.get(struct_name) else {
        return Ok(None);
    };

    let mut candidate: Option<PromotedPointField> = None;
    for (point_field_name, point_type_name) in point_fields {
        let Some(point_field) = owner_info.get_field(point_field_name) else {
            continue;
        };

        let promoted_struct = codegen.resolve_struct_type_name(point_type_name);
        let Some(promoted_info) = codegen.type_registry.get(&promoted_struct) else {
            continue;
        };
        let Some(promoted_field) = promoted_info.get_field(field_name) else {
            continue;
        };

        let next = (
            promoted_struct,
            promoted_field.type_name.clone(),
            point_field.offset,
            promoted_field.offset,
            point_field_name.clone(),
        );
        if candidate.is_some() {
            return Err(anyhow!(
                "Ambiguous promoted field '{}.{}' from multiple point fields",
                struct_name,
                field_name
            ));
        }
        candidate = Some(next);
    }

    Ok(candidate)
}

pub(super) fn resolve_promoted_point_method(
    codegen: &CodeGen,
    struct_name: &str,
    method_name: &str,
) -> Result<Option<(u64, String, String, String)>> {
    let Some(point_fields) = codegen.struct_point_fields.get(struct_name) else {
        return Ok(None);
    };
    let Some(owner_info) = codegen.type_registry.get(struct_name) else {
        return Ok(None);
    };

    let mut candidate: Option<(u64, String, String, String)> = None;
    for (point_field_name, point_type_name) in point_fields {
        let Some(point_field) = owner_info.get_field(point_field_name) else {
            continue;
        };

        let promoted_struct = codegen.resolve_struct_type_name(point_type_name);
        let Some(func_name) =
            codegen.try_resolve_struct_method_name(&promoted_struct, method_name)?
        else {
            continue;
        };

        let next = (
            point_field.offset,
            func_name,
            point_field_name.clone(),
            promoted_struct,
        );
        if candidate.is_some() {
            return Err(anyhow!(
                "Ambiguous promoted method '{}.{}' from multiple point fields",
                struct_name,
                method_name
            ));
        }
        candidate = Some(next);
    }

    Ok(candidate)
}

pub(super) fn infer_builtin_method_result_type(
    receiver_type: &VarType,
    method_name: &str,
    arg_count: usize,
) -> Option<VarType> {
    match receiver_type {
        VarType::String => {
            if method_name == "len" && arg_count == 0 {
                Some(VarType::Int)
            } else {
                None
            }
        }
        VarType::DynamicArray(elem_type) => match (method_name, arg_count) {
            ("len", 0) => Some(VarType::Int),
            ("push", 1) => Some(VarType::DynamicArray(elem_type.clone())),
            ("pop", 0) => Some((**elem_type).clone()),
            ("get", 1) => Some((**elem_type).clone()),
            _ => None,
        },
        VarType::StaticArray(_, _) => {
            if method_name == "len" && arg_count == 0 {
                Some(VarType::Int)
            } else {
                None
            }
        }
        VarType::Map(_, value_type) => match (method_name, arg_count) {
            ("len", 0) => Some(VarType::Int),
            ("get", 1) => Some((**value_type).clone()),
            ("set", 2) => Some(VarType::Int),
            ("contains", 1) => Some(VarType::Int),
            ("delete", 1) => Some(VarType::Int),
            _ => None,
        },
        VarType::Set(_) => match (method_name, arg_count) {
            ("len", 0) => Some(VarType::Int),
            ("add", 1) => Some(VarType::Int),
            ("contains", 1) => Some(VarType::Int),
            ("remove", 1) => Some(VarType::Int),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn compile_struct_method_call(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    receiver_struct_name: Option<&str>,
    self_ptr: Value,
    func_name: &str,
    args: &[Expression],
) -> Result<Value> {
    validate_generic_constraints_for_call(codegen, ctx, func_name, args)?;
    let func_id = *codegen
        .functions
        .get(func_name)
        .ok_or_else(|| anyhow!("Method not found: {}", func_name))?;

    let mut adjusted_self = self_ptr;
    if let Some(receiver_struct_name) = receiver_struct_name {
        let method_owner = codegen
            .function_param_types
            .get(func_name)
            .and_then(|params| params.first())
            .and_then(|ty| match ty {
                VarType::Struct(name) => Some(name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| receiver_struct_name.to_string());

        let receiver_struct = codegen.resolve_struct_type_name(receiver_struct_name);
        if receiver_struct != method_owner {
            let receiver_info = codegen
                .type_registry
                .get(&receiver_struct)
                .ok_or_else(|| anyhow!("Unknown struct type: {}", receiver_struct))?;
            let owner_info = codegen
                .type_registry
                .get(&method_owner)
                .ok_or_else(|| anyhow!("Unknown struct type: {}", method_owner))?;

            if let Some(owner_first_field) = owner_info.fields.first() {
                let receiver_field = receiver_info
                    .get_field(&owner_first_field.name)
                    .ok_or_else(|| {
                        anyhow!(
                            "Struct '{}' does not contain embedded layout for '{}'",
                            receiver_struct,
                            method_owner
                        )
                    })?;
                let delta = receiver_field.offset as i64 - owner_first_field.offset as i64;
                if delta != 0 {
                    adjusted_self = builder.ins().iadd_imm(self_ptr, delta);
                }
            }
        }
    }

    let mut arg_vals = vec![adjusted_self];
    let expected_params = codegen.function_param_types.get(func_name).cloned();
    for (idx, arg) in args.iter().enumerate() {
        let expected = expected_params
            .as_ref()
            .and_then(|params| params.get(idx + 1));
        arg_vals.push(compile_expression_with_type_hint(
            codegen, builder, ctx, arg, expected,
        )?);
    }

    let func_ref = codegen.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &arg_vals);
    let results = builder.inst_results(call);

    if results.is_empty() {
        Ok(builder.ins().iconst(types::I64, 0))
    } else {
        Ok(results[0])
    }
}

pub(super) fn compile_builtin_method(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    receiver: Value,
    var_type: &VarType,
    method_name: &str,
    args: &[Expression],
) -> Result<Option<Value>> {
    match var_type {
        VarType::String => {
            if method_name == "len" {
                let res = runtime::call_runtime(codegen, builder, "breom_string_len", &[receiver])?;
                return Ok(Some(res));
            }
        }
        VarType::DynamicArray(_) => match method_name {
            "len" => {
                return Ok(Some(builder.ins().load(
                    types::I64,
                    MemFlags::new(),
                    receiver,
                    ARRAY_LEN_OFFSET,
                )));
            }
            "push" => {
                if args.len() != 1 {
                    return Err(anyhow!("push requires 1 argument"));
                }
                let val = compile_expression(codegen, builder, ctx, &args[0])?;
                let new_ptr =
                    runtime::call_runtime(codegen, builder, "breom_array_push", &[receiver, val])?;
                return Ok(Some(new_ptr));
            }
            "pop" => {
                let res = runtime::call_runtime(codegen, builder, "breom_array_pop", &[receiver])?;
                return Ok(Some(res));
            }
            "get" => {
                if args.len() != 1 {
                    return Err(anyhow!("get requires 1 argument"));
                }
                let idx = compile_expression(codegen, builder, ctx, &args[0])?;
                let elem_ptr = compile_array_index_ptr(builder, receiver, idx);
                return Ok(Some(builder.ins().load(
                    types::I64,
                    MemFlags::new(),
                    elem_ptr,
                    0,
                )));
            }
            _ => {}
        },
        VarType::StaticArray(_, _) => {
            if method_name == "len" {
                return Ok(Some(builder.ins().load(
                    types::I64,
                    MemFlags::new(),
                    receiver,
                    ARRAY_LEN_OFFSET,
                )));
            }
        }
        VarType::Map(_, _) => match method_name {
            "len" => {
                let res = runtime::call_runtime(codegen, builder, "breom_map_len", &[receiver])?;
                return Ok(Some(res));
            }
            "get" => {
                if args.len() != 1 {
                    return Err(anyhow!("get requires 1 argument"));
                }
                let key = compile_expression(codegen, builder, ctx, &args[0])?;
                let res =
                    runtime::call_runtime(codegen, builder, "breom_map_get", &[receiver, key])?;
                return Ok(Some(res));
            }
            "set" => {
                if args.len() != 2 {
                    return Err(anyhow!("set requires 2 arguments"));
                }
                let key = compile_expression(codegen, builder, ctx, &args[0])?;
                let val = compile_expression(codegen, builder, ctx, &args[1])?;
                runtime::call_runtime(codegen, builder, "breom_map_set", &[receiver, key, val])?;
                return Ok(Some(builder.ins().iconst(types::I64, 0)));
            }
            "contains" => {
                if args.len() != 1 {
                    return Err(anyhow!("contains requires 1 argument"));
                }
                let key = compile_expression(codegen, builder, ctx, &args[0])?;
                let res = runtime::call_runtime(
                    codegen,
                    builder,
                    "breom_map_contains",
                    &[receiver, key],
                )?;
                return Ok(Some(res));
            }
            "delete" => {
                if args.len() != 1 {
                    return Err(anyhow!("delete requires 1 argument"));
                }
                let key = compile_expression(codegen, builder, ctx, &args[0])?;
                let res =
                    runtime::call_runtime(codegen, builder, "breom_map_delete", &[receiver, key])?;
                return Ok(Some(res));
            }
            _ => {}
        },
        VarType::Set(_) => match method_name {
            "len" => {
                let res = runtime::call_runtime(codegen, builder, "breom_set_len", &[receiver])?;
                return Ok(Some(res));
            }
            "add" => {
                if args.len() != 1 {
                    return Err(anyhow!("add requires 1 argument"));
                }
                let val = compile_expression(codegen, builder, ctx, &args[0])?;
                runtime::call_runtime(codegen, builder, "breom_set_add", &[receiver, val])?;
                return Ok(Some(builder.ins().iconst(types::I64, 0)));
            }
            "contains" => {
                if args.len() != 1 {
                    return Err(anyhow!("contains requires 1 argument"));
                }
                let val = compile_expression(codegen, builder, ctx, &args[0])?;
                let res = runtime::call_runtime(
                    codegen,
                    builder,
                    "breom_set_contains",
                    &[receiver, val],
                )?;
                return Ok(Some(res));
            }
            "remove" => {
                if args.len() != 1 {
                    return Err(anyhow!("remove requires 1 argument"));
                }
                let val = compile_expression(codegen, builder, ctx, &args[0])?;
                let res =
                    runtime::call_runtime(codegen, builder, "breom_set_remove", &[receiver, val])?;
                return Ok(Some(res));
            }
            _ => {}
        },
        VarType::Struct(struct_name) => {
            let resolved = codegen.resolve_struct_type_name(struct_name);
            if resolved == "file.io.Reader" {
                match method_name {
                    "read_all" => {
                        if !args.is_empty() {
                            return Err(anyhow!("read_all requires 0 arguments"));
                        }
                        let res = runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_file_reader_read_all",
                            &[receiver],
                        )?;
                        return Ok(Some(res));
                    }
                    "close" => {
                        if !args.is_empty() {
                            return Err(anyhow!("close requires 0 arguments"));
                        }
                        let res = runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_file_reader_close",
                            &[receiver],
                        )?;
                        return Ok(Some(res));
                    }
                    _ => {}
                }
            }

            if resolved == "file.io.Scanner" {
                match method_name {
                    "has_next" => {
                        if !args.is_empty() {
                            return Err(anyhow!("has_next requires 0 arguments"));
                        }
                        let res = runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_file_scanner_has_next",
                            &[receiver],
                        )?;
                        return Ok(Some(res));
                    }
                    "next_line" => {
                        if !args.is_empty() {
                            return Err(anyhow!("next_line requires 0 arguments"));
                        }
                        let res = runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_file_scanner_next_line",
                            &[receiver],
                        )?;
                        return Ok(Some(res));
                    }
                    "close" => {
                        if !args.is_empty() {
                            return Err(anyhow!("close requires 0 arguments"));
                        }
                        let res = runtime::call_runtime(
                            codegen,
                            builder,
                            "breom_file_scanner_close",
                            &[receiver],
                        )?;
                        return Ok(Some(res));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

pub(super) fn compile_indirect_call(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    func_ptr: Value,
    func_type: Option<&VarType>,
    args: &[Expression],
) -> Result<Value> {
    let lambda_sig = match func_type {
        Some(VarType::Lambda {
            params,
            return_type,
        }) => Some((params.clone(), (*return_type.clone()))),
        _ => None,
    };

    let mut arg_vals = Vec::with_capacity(args.len());
    for (idx, arg) in args.iter().enumerate() {
        let expected_ty = lambda_sig.as_ref().and_then(|(params, _)| params.get(idx));
        arg_vals.push(compile_expression_with_type_hint(
            codegen,
            builder,
            ctx,
            arg,
            expected_ty,
        )?);
    }

    let mut sig = codegen.module.make_signature();
    if let Some((params, return_type)) = &lambda_sig {
        for param_ty in params {
            sig.params
                .push(AbiParam::new(clif_type_from_var_type(param_ty)));
        }
        sig.returns
            .push(AbiParam::new(clif_type_from_var_type(return_type)));
    } else {
        for _ in args {
            sig.params.push(AbiParam::new(types::I64));
        }
        sig.returns.push(AbiParam::new(types::I64));
    }
    let sig_ref = builder.import_signature(sig);

    let call = builder.ins().call_indirect(sig_ref, func_ptr, &arg_vals);
    let results = builder.inst_results(call);
    Ok(if results.is_empty() {
        builder.ins().iconst(types::I64, 0)
    } else {
        results[0]
    })
}
