use super::*;

pub(super) fn compile_return_stmt(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    ret_stmt: &ReturnStmt,
) -> Result<Option<Value>> {
    let expected_return_type = ctx.expected_return_type.clone();
    let return_val = if let Some(ref expr) = ret_stmt.value {
        Some(expr::compile_expression_with_type_hint(
            codegen,
            builder,
            ctx,
            expr,
            expected_return_type.as_ref(),
        )?)
    } else {
        None
    };
    let return_var_name = ret_stmt.value.as_ref().and_then(|e| {
        if let Expression::Identifier(name, _) = e {
            Some(name.clone())
        } else {
            None
        }
    });
    if let Some(ref var_name) = return_var_name {
        ctx.remove_heap_var(var_name);
    }
    runtime::release_all_heap_vars(codegen, builder, ctx)?;
    execute_defers(codegen, builder, ctx)?;
    if ctx.is_error_result {
        let zero = builder.ins().iconst(types::I64, 0);
        let val = return_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
        builder.ins().return_(&[zero, val]);
    } else if let Some(val) = return_val {
        builder.ins().return_(&[val]);
    } else {
        builder.ins().return_(&[]);
    }

    let dead_block = builder.create_block();
    builder.switch_to_block(dead_block);
    builder.seal_block(dead_block);
    Ok(None)
}

pub(super) fn compile_throw_stmt(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    expr_stmt: &Expression,
) -> Result<Option<Value>> {
    let err_val = expr::compile_expression(codegen, builder, ctx, expr_stmt)?;
    let zero = builder.ins().iconst(types::I64, 0);
    runtime::release_all_heap_vars(codegen, builder, ctx)?;
    execute_defers(codegen, builder, ctx)?;
    builder.ins().return_(&[err_val, zero]);

    let dead_block = builder.create_block();
    builder.switch_to_block(dead_block);
    builder.seal_block(dead_block);
    Ok(None)
}

pub(super) fn compile_var_decl_stmt(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    var_decl: &crate::ast::declarations::VarDecl,
) -> Result<Option<Value>> {
    let inferred_value_type = typing::infer_expr_type(codegen, ctx, &var_decl.value);
    let var_type = if let Some(type_expr) = &var_decl.type_annotation {
        typing::infer_type_expr_to_var_type_with_codegen(codegen, type_expr)
    } else {
        inferred_value_type.clone()
    };

    let uses_static_array_context = matches!(
        (&var_type, &var_decl.value),
        (
            VarType::StaticArray(_, _),
            Expression::Collection(
                CollectionLiteral::DynamicArray(_, _) | CollectionLiteral::RepeatedArray { .. }
            )
        )
    );

    if var_decl.type_annotation.is_some()
        && !uses_static_array_context
        && !is_type_assignable(codegen, &var_type, &inferred_value_type)
    {
        return Err(anyhow!(
            "Type mismatch for '{}': expected {:?}, got {:?}",
            var_decl.name,
            var_type,
            inferred_value_type
        ));
    }

    if let VarType::StaticArray(_, declared_len) = &var_type {
        if let Expression::Collection(CollectionLiteral::DynamicArray(elements, _)) =
            &var_decl.value
        {
            if elements.len() > *declared_len {
                return Err(anyhow!(
                    "Static array length mismatch for '{}': declared {}, got {}",
                    var_decl.name,
                    declared_len,
                    elements.len()
                ));
            }
        }
    }

    let is_heap = var_type.is_heap_type();

    let val = expr::compile_expression_with_type_hint(
        codegen,
        builder,
        ctx,
        &var_decl.value,
        Some(&var_type),
    )?;

    if is_heap && is_borrowed_heap_expression(&var_decl.value) {
        runtime::arc_retain(codegen, builder, val)?;
    }

    if ctx.variables.contains_key(&var_decl.name) {
        let old_type = ctx.get_var_type(&var_decl.name);
        if old_type.is_heap_type() {
            runtime::release_var(codegen, builder, ctx, &var_decl.name)?;
        }

        ctx.set_variable(builder, &var_decl.name, val)?;
        ctx.set_var_type(&var_decl.name, var_type);
    } else {
        let decl_clif_ty = match &var_type {
            VarType::Float => types::F64,
            _ => types::I64,
        };
        let var = ctx.create_variable(builder, &var_decl.name, decl_clif_ty);
        builder.def_var(var, val);
        ctx.set_var_type(&var_decl.name, var_type);

        if is_heap {
            ctx.register_heap_var(&var_decl.name);
        }
    }
    Ok(Some(val))
}

pub(super) fn compile_expression_stmt(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    expr_stmt: &Expression,
) -> Result<Option<Value>> {
    let val = expr::compile_expression(codegen, builder, ctx, expr_stmt)?;
    Ok(Some(val))
}

pub(super) fn compile_assignment_stmt(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    assign: &AssignmentStmt,
) -> Result<Option<Value>> {
    let old_type = ctx.get_var_type(&assign.target.base);

    if !assign.target.accessors.is_empty() {
        let mut owner = ctx.get_variable(builder, &assign.target.base)?;
        let mut owner_type = old_type.clone();

        for accessor in assign
            .target
            .accessors
            .iter()
            .take(assign.target.accessors.len().saturating_sub(1))
        {
            match accessor {
                Accessor::Member(field_name) => {
                    let struct_name = match &owner_type {
                        VarType::Struct(name) => name.clone(),
                        _ => {
                            return Err(anyhow!(
                                "Member access on non-struct lvalue: {}",
                                field_name
                            ));
                        }
                    };

                    let resolved_struct_name = codegen.resolve_struct_type_name(&struct_name);
                    let type_info = codegen
                        .type_registry
                        .get(&resolved_struct_name)
                        .ok_or_else(|| anyhow!("Unknown struct type: {}", resolved_struct_name))?;
                    let field = type_info.get_field(field_name).ok_or_else(|| {
                        anyhow!("Unknown field: {}.{}", resolved_struct_name, field_name)
                    })?;
                    codegen.check_field_access(&resolved_struct_name, field_name)?;

                    let field_ptr = builder.ins().iadd_imm(owner, field.offset as i64);
                    let field_var_ty = var_type_from_type_name(&field.type_name);
                    let field_ty = clif_type_from_var_type(&field_var_ty);
                    owner = builder.ins().load(field_ty, MemFlags::new(), field_ptr, 0);
                    owner_type = field_var_ty;
                }
                Accessor::Index(index_expr) => {
                    let index = expr::compile_expression(codegen, builder, ctx, index_expr)?;
                    match owner_type {
                        VarType::DynamicArray(inner) => {
                            let elem_ptr = compile_array_index_ptr(builder, owner, index);
                            owner = builder.ins().load(types::I64, MemFlags::new(), elem_ptr, 0);
                            owner_type = *inner;
                        }
                        VarType::StaticArray(inner, _) => {
                            let elem_ptr = compile_array_index_ptr(builder, owner, index);
                            owner = builder.ins().load(types::I64, MemFlags::new(), elem_ptr, 0);
                            owner_type = *inner;
                        }
                        VarType::Unknown => {
                            owner = runtime::call_runtime(
                                codegen,
                                builder,
                                "breom_array_get",
                                &[owner, index],
                            )?;
                            owner_type = VarType::Unknown;
                        }
                        _ => {
                            return Err(anyhow!(
                                "Index access on non-array lvalue: {} ({:?})",
                                assign.target.base,
                                owner_type
                            ));
                        }
                    }
                }
            }
        }

        let last_accessor =
            assign.target.accessors.last().ok_or_else(|| {
                anyhow!("Invalid lvalue accessor chain for {}", assign.target.base)
            })?;

        let assigned = match last_accessor {
            Accessor::Member(field_name) => {
                let struct_name = match &owner_type {
                    VarType::Struct(name) => name.clone(),
                    _ => {
                        return Err(anyhow!(
                            "Member assignment on non-struct lvalue: {}",
                            field_name
                        ));
                    }
                };

                let resolved_struct_name = codegen.resolve_struct_type_name(&struct_name);
                let type_info = codegen
                    .type_registry
                    .get(&resolved_struct_name)
                    .ok_or_else(|| anyhow!("Unknown struct type: {}", resolved_struct_name))?;
                let field = type_info.get_field(field_name).ok_or_else(|| {
                    anyhow!("Unknown field: {}.{}", resolved_struct_name, field_name)
                })?;
                codegen.check_field_access(&resolved_struct_name, field_name)?;

                let field_ptr = builder.ins().iadd_imm(owner, field.offset as i64);
                let field_var_ty = var_type_from_type_name(&field.type_name);
                let field_ty = clif_type_from_var_type(&field_var_ty);

                if assign.op == AssignOp::Assign {
                    let rhs_type = typing::infer_expr_type(codegen, ctx, &assign.value);
                    if !is_type_assignable(codegen, &field_var_ty, &rhs_type) {
                        return Err(anyhow!(
                            "Type mismatch for '{}.{}': expected {:?}, got {:?}",
                            resolved_struct_name,
                            field_name,
                            field_var_ty,
                            rhs_type
                        ));
                    }
                    let rhs = expr::compile_expression_with_type_hint(
                        codegen,
                        builder,
                        ctx,
                        &assign.value,
                        Some(&field_var_ty),
                    )?;
                    builder.ins().store(MemFlags::new(), rhs, field_ptr, 0);
                    rhs
                } else {
                    let lhs = builder.ins().load(field_ty, MemFlags::new(), field_ptr, 0);
                    let rhs = expr::compile_expression(codegen, builder, ctx, &assign.value)?;
                    let new_val = apply_assign_op(codegen, builder, assign.op, lhs, rhs)?;
                    builder.ins().store(MemFlags::new(), new_val, field_ptr, 0);
                    new_val
                }
            }
            Accessor::Index(index_expr) => {
                let index = expr::compile_expression(codegen, builder, ctx, index_expr)?;
                match owner_type {
                    VarType::DynamicArray(elem_ty) | VarType::StaticArray(elem_ty, _) => {
                        let elem_ptr = compile_array_index_ptr(builder, owner, index);
                        if assign.op == AssignOp::Assign {
                            let rhs_type = typing::infer_expr_type(codegen, ctx, &assign.value);
                            if !is_type_assignable(codegen, &elem_ty, &rhs_type) {
                                return Err(anyhow!(
                                    "Type mismatch for indexed assignment: expected {:?}, got {:?}",
                                    elem_ty,
                                    rhs_type
                                ));
                            }
                            let rhs =
                                expr::compile_expression(codegen, builder, ctx, &assign.value)?;
                            builder.ins().store(MemFlags::new(), rhs, elem_ptr, 0);
                            rhs
                        } else {
                            let lhs = builder.ins().load(types::I64, MemFlags::new(), elem_ptr, 0);
                            let rhs =
                                expr::compile_expression(codegen, builder, ctx, &assign.value)?;
                            let new_val = apply_assign_op(codegen, builder, assign.op, lhs, rhs)?;
                            builder.ins().store(MemFlags::new(), new_val, elem_ptr, 0);
                            new_val
                        }
                    }
                    VarType::Unknown => {
                        if assign.op == AssignOp::Assign {
                            let rhs =
                                expr::compile_expression(codegen, builder, ctx, &assign.value)?;
                            runtime::call_runtime(
                                codegen,
                                builder,
                                "breom_array_set",
                                &[owner, index, rhs],
                            )?;
                            rhs
                        } else {
                            let lhs = runtime::call_runtime(
                                codegen,
                                builder,
                                "breom_array_get",
                                &[owner, index],
                            )?;
                            let rhs =
                                expr::compile_expression(codegen, builder, ctx, &assign.value)?;
                            let new_val = apply_assign_op(codegen, builder, assign.op, lhs, rhs)?;
                            runtime::call_runtime(
                                codegen,
                                builder,
                                "breom_array_set",
                                &[owner, index, new_val],
                            )?;
                            new_val
                        }
                    }
                    _ => {
                        return Err(anyhow!(
                            "Index assignment on non-array lvalue: {} ({:?})",
                            assign.target.base,
                            owner_type
                        ));
                    }
                }
            }
        };

        return Ok(Some(assigned));
    }

    let val = if assign.op == AssignOp::Assign {
        let rhs_type = typing::infer_expr_type(codegen, ctx, &assign.value);
        let uses_static_array_context = matches!(
            (&old_type, &assign.value),
            (
                VarType::StaticArray(_, _),
                Expression::Collection(
                    CollectionLiteral::DynamicArray(_, _) | CollectionLiteral::RepeatedArray { .. }
                )
            )
        );
        if !uses_static_array_context && !is_type_assignable(codegen, &old_type, &rhs_type) {
            return Err(anyhow!(
                "Type mismatch for '{}': expected {:?}, got {:?}",
                assign.target.base,
                old_type,
                rhs_type
            ));
        }

        let old_heap_val = if old_type.is_heap_type() {
            Some(ctx.get_variable(builder, &assign.target.base)?)
        } else {
            None
        };

        let expected_type = if matches!(old_type, VarType::StaticArray(_, _)) {
            Some(&old_type)
        } else {
            None
        };
        let new_val = expr::compile_expression_with_type_hint(
            codegen,
            builder,
            ctx,
            &assign.value,
            expected_type,
        )?;

        if old_type.is_heap_type() && is_borrowed_heap_expression(&assign.value) {
            if let Some(old_val) = old_heap_val {
                let same_ptr = builder.ins().icmp(IntCC::Equal, old_val, new_val);
                let retain_block = builder.create_block();
                let skip_retain_block = builder.create_block();

                builder
                    .ins()
                    .brif(same_ptr, skip_retain_block, &[], retain_block, &[]);

                builder.switch_to_block(retain_block);
                builder.seal_block(retain_block);
                runtime::arc_retain(codegen, builder, new_val)?;
                builder.ins().jump(skip_retain_block, &[]);

                builder.switch_to_block(skip_retain_block);
                builder.seal_block(skip_retain_block);
            } else {
                runtime::arc_retain(codegen, builder, new_val)?;
            }
        }

        if let Some(old_val) = old_heap_val {
            let same_ptr = builder.ins().icmp(IntCC::Equal, old_val, new_val);
            let skip_release_block = builder.create_block();
            let release_block = builder.create_block();
            let continue_block = builder.create_block();

            builder
                .ins()
                .brif(same_ptr, skip_release_block, &[], release_block, &[]);

            builder.switch_to_block(release_block);
            builder.seal_block(release_block);
            runtime::call_runtime(codegen, builder, "breom_arc_release", &[old_val])?;
            builder.ins().jump(continue_block, &[]);

            builder.switch_to_block(skip_release_block);
            builder.seal_block(skip_release_block);
            builder.ins().jump(continue_block, &[]);

            builder.switch_to_block(continue_block);
            builder.seal_block(continue_block);
        }

        new_val
    } else {
        let lhs = ctx.get_variable(builder, &assign.target.base)?;
        let rhs = expr::compile_expression(codegen, builder, ctx, &assign.value)?;
        apply_assign_op(codegen, builder, assign.op, lhs, rhs)?
    };
    ctx.set_variable(builder, &assign.target.base, val)?;

    let new_type = if assign.op == AssignOp::Assign {
        if matches!(old_type, VarType::StaticArray(_, _)) {
            old_type.clone()
        } else {
            typing::infer_expr_type(codegen, ctx, &assign.value)
        }
    } else {
        old_type
    };
    ctx.set_var_type(&assign.target.base, new_type);

    Ok(Some(val))
}

pub(super) fn compile_break_stmt(
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
) -> Result<Option<Value>> {
    let exit_block = ctx
        .current_loop_exit()
        .ok_or_else(|| anyhow!("break outside of loop"))?;
    builder.ins().jump(exit_block, &[]);

    let dead_block = builder.create_block();
    builder.switch_to_block(dead_block);
    builder.seal_block(dead_block);
    Ok(None)
}

pub(super) fn compile_continue_stmt(
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
) -> Result<Option<Value>> {
    let header_block = ctx
        .current_loop_header()
        .ok_or_else(|| anyhow!("continue outside of loop"))?;
    builder.ins().jump(header_block, &[]);

    let dead_block = builder.create_block();
    builder.switch_to_block(dead_block);
    builder.seal_block(dead_block);
    Ok(None)
}

pub(super) fn compile_defer_stmt(
    ctx: &mut FunctionContext,
    defer_stmt: &DeferStmt,
) -> Result<Option<Value>> {
    ctx.push_defer(defer_stmt.body.clone());
    Ok(None)
}

pub(super) fn compile_instead_stmt(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    expr_stmt: &Expression,
) -> Result<Option<Value>> {
    let val = expr::compile_expression(codegen, builder, ctx, expr_stmt)?;
    Ok(Some(val))
}
