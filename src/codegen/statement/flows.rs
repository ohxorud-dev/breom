use super::*;

pub(super) fn compile_if(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    if_stmt: &IfStmt,
) -> Result<()> {
    let cond_val = expr::compile_expression(codegen, builder, ctx, &if_stmt.condition)?;

    let then_block = builder.create_block();
    let merge_block = builder.create_block();
    let else_block = if if_stmt.else_clause.is_some() {
        builder.create_block()
    } else {
        merge_block
    };

    let zero = builder.ins().iconst(types::I64, 0);
    let cond = builder.ins().icmp(IntCC::NotEqual, cond_val, zero);
    builder.ins().brif(cond, then_block, &[], else_block, &[]);

    builder.switch_to_block(then_block);
    builder.seal_block(then_block);

    ctx.enter_scope();

    let mut then_returned = false;
    for stmt in &if_stmt.then_block.statements {
        if let Statement::Return(_) | Statement::Throw(..) = stmt {
            then_returned = true;
        }
        compile_statement(codegen, builder, ctx, stmt)?;
        if then_returned {
            break;
        }
    }
    if !then_returned {
        runtime::release_scope_vars(codegen, builder, ctx)?;
        builder.ins().jump(merge_block, &[]);
    } else {
        ctx.exit_scope();
    }

    if let Some(ref else_clause) = if_stmt.else_clause {
        builder.switch_to_block(else_block);
        builder.seal_block(else_block);

        ctx.enter_scope();

        let mut else_returned = false;
        match else_clause {
            ElseClause::Else(block) => {
                for stmt in &block.statements {
                    if let Statement::Return(_) | Statement::Throw(..) = stmt {
                        else_returned = true;
                    }
                    compile_statement(codegen, builder, ctx, stmt)?;
                    if else_returned {
                        break;
                    }
                }
            }
            ElseClause::ElseIf(nested_if) => {
                compile_if(codegen, builder, ctx, nested_if)?;
            }
        }
        if !else_returned {
            if !matches!(else_clause, ElseClause::ElseIf(_)) {
                runtime::release_scope_vars(codegen, builder, ctx)?;
                builder.ins().jump(merge_block, &[]);
            } else {
                ctx.exit_scope();
            }
        } else {
            ctx.exit_scope();
        }
    }

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(())
}

pub(super) fn compile_for(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    for_stmt: &ForStmt,
) -> Result<()> {
    match for_stmt {
        ForStmt::Infinite(block, _) => {
            let loop_block = builder.create_block();
            let exit_block = builder.create_block();

            builder.ins().jump(loop_block, &[]);
            builder.switch_to_block(loop_block);

            ctx.push_loop(loop_block, exit_block);

            for stmt in &block.statements {
                compile_statement(codegen, builder, ctx, stmt)?;
            }

            builder.ins().jump(loop_block, &[]);
            builder.seal_block(loop_block);

            ctx.pop_loop();

            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
        }

        ForStmt::Condition(cond_expr, block, _) => {
            let header_block = builder.create_block();
            let body_block = builder.create_block();
            let exit_block = builder.create_block();

            builder.ins().jump(header_block, &[]);
            builder.switch_to_block(header_block);

            let cond_val = expr::compile_expression(codegen, builder, ctx, cond_expr)?;
            let zero = builder.ins().iconst(types::I64, 0);
            let cond = builder.ins().icmp(IntCC::NotEqual, cond_val, zero);
            builder.ins().brif(cond, body_block, &[], exit_block, &[]);

            builder.switch_to_block(body_block);
            builder.seal_block(body_block);

            ctx.push_loop(header_block, exit_block);

            for stmt in &block.statements {
                compile_statement(codegen, builder, ctx, stmt)?;
            }

            builder.ins().jump(header_block, &[]);
            builder.seal_block(header_block);

            ctx.pop_loop();

            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
        }

        ForStmt::Count(count, block, _) => {
            let header_block = builder.create_block();
            let body_block = builder.create_block();
            let continue_block = builder.create_block();
            let exit_block = builder.create_block();

            let counter_name = format!("__for_count_{}__", ctx.variables.len());
            let counter_var = ctx.create_variable(builder, &counter_name, types::I64);
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(counter_var, zero);

            builder.ins().jump(header_block, &[]);
            builder.switch_to_block(header_block);

            let current = builder.use_var(counter_var);
            let limit = builder.ins().iconst(types::I64, *count as i64);
            let cond = builder.ins().icmp(IntCC::SignedLessThan, current, limit);
            builder.ins().brif(cond, body_block, &[], exit_block, &[]);

            builder.switch_to_block(body_block);
            builder.seal_block(body_block);

            ctx.push_loop(continue_block, exit_block);

            for stmt in &block.statements {
                compile_statement(codegen, builder, ctx, stmt)?;
            }

            builder.ins().jump(continue_block, &[]);

            builder.switch_to_block(continue_block);
            builder.seal_block(continue_block);

            let current = builder.use_var(counter_var);
            let one = builder.ins().iconst(types::I64, 1);
            let next = builder.ins().iadd(current, one);
            builder.def_var(counter_var, next);

            builder.ins().jump(header_block, &[]);
            builder.seal_block(header_block);

            ctx.pop_loop();

            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
        }

        ForStmt::Range(range_for) => {
            let header_block = builder.create_block();
            let body_block = builder.create_block();
            let continue_block = builder.create_block();
            let exit_block = builder.create_block();

            let iterable = expr::compile_expression(codegen, builder, ctx, &range_for.iterable)?;

            let idx_var = if range_for.index_var == "_" {
                builder.declare_var(types::I64)
            } else {
                let var = ctx.create_variable(builder, &range_for.index_var, types::I64);
                ctx.set_var_type(&range_for.index_var, VarType::Int);
                var
            };
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(idx_var, zero);

            let iter_type = if let Expression::Identifier(var_name, _) = &range_for.iterable {
                ctx.get_var_type(var_name)
            } else {
                typing::infer_expr_type(codegen, ctx, &range_for.iterable)
            };

            let length = match &iter_type {
                VarType::DynamicArray(_) => {
                    builder
                        .ins()
                        .load(types::I64, MemFlags::new(), iterable, ARRAY_LEN_OFFSET)
                }
                VarType::StaticArray(_, len) => builder.ins().iconst(types::I64, *len as i64),
                VarType::Int => iterable,
                VarType::Unknown => {
                    runtime::call_runtime(codegen, builder, "breom_array_len", &[iterable])?
                }
                _ => {
                    return Err(anyhow!("range expects Int or Array, got {:?}", iter_type));
                }
            };

            if matches!(iter_type, VarType::Int) && range_for.value_var.is_some() {
                return Err(anyhow!(
                    "range Int supports index-only form: for i := range n"
                ));
            }

            let value_var = if let Some(ref val_name) = range_for.value_var {
                let var = ctx.create_variable(builder, val_name, types::I64);
                let value_type = match &iter_type {
                    VarType::DynamicArray(inner) => (**inner).clone(),
                    VarType::StaticArray(inner, _) => (**inner).clone(),
                    _ => VarType::Unknown,
                };
                ctx.set_var_type(val_name, value_type);
                Some(var)
            } else {
                None
            };

            let array_elem_ptr_var = if value_var.is_some()
                && matches!(
                    iter_type,
                    VarType::DynamicArray(_) | VarType::StaticArray(_, _)
                ) {
                let var = builder.declare_var(types::I64);
                let data_ptr = builder.ins().iadd_imm(iterable, ARRAY_DATA_OFFSET);
                builder.def_var(var, data_ptr);
                Some(var)
            } else {
                None
            };

            let use_ptr_end_loop = false;

            let array_end_ptr_var = if use_ptr_end_loop {
                let var = builder.declare_var(types::I64);
                let data_ptr = builder.ins().iadd_imm(iterable, ARRAY_DATA_OFFSET);
                let len_bytes = builder.ins().ishl_imm(length, 3);
                let end_ptr = builder.ins().iadd(data_ptr, len_bytes);
                builder.def_var(var, end_ptr);
                Some(var)
            } else {
                None
            };

            builder.ins().jump(header_block, &[]);
            builder.switch_to_block(header_block);

            let cond = if use_ptr_end_loop {
                let elem_ptr_var = array_elem_ptr_var.expect("array elem pointer must exist");
                let end_ptr_var = array_end_ptr_var.expect("array end pointer must exist");
                let elem_ptr = builder.use_var(elem_ptr_var);
                let end_ptr = builder.use_var(end_ptr_var);
                builder
                    .ins()
                    .icmp(IntCC::UnsignedLessThan, elem_ptr, end_ptr)
            } else {
                let current_idx = builder.use_var(idx_var);
                builder
                    .ins()
                    .icmp(IntCC::SignedLessThan, current_idx, length)
            };
            builder.ins().brif(cond, body_block, &[], exit_block, &[]);

            builder.switch_to_block(body_block);
            builder.seal_block(body_block);

            ctx.push_loop(continue_block, exit_block);

            if let Some(val_var) = value_var {
                let current_idx = builder.use_var(idx_var);
                let value = match &iter_type {
                    VarType::DynamicArray(_) | VarType::StaticArray(_, _) => {
                        let elem_ptr_var = array_elem_ptr_var
                            .expect("array_elem_ptr_var must exist for array iteration");
                        let elem_ptr = builder.use_var(elem_ptr_var);
                        builder.ins().load(types::I64, MemFlags::new(), elem_ptr, 0)
                    }
                    VarType::Unknown => runtime::call_runtime(
                        codegen,
                        builder,
                        "breom_array_get",
                        &[iterable, current_idx],
                    )?,
                    _ => {
                        return Err(anyhow!(
                            "range value access expects Array, got {:?}",
                            iter_type
                        ));
                    }
                };
                builder.def_var(val_var, value);
            }

            for stmt in &range_for.body.statements {
                compile_statement(codegen, builder, ctx, stmt)?;
            }

            builder.ins().jump(continue_block, &[]);

            builder.switch_to_block(continue_block);
            builder.seal_block(continue_block);

            if !use_ptr_end_loop {
                let current_idx = builder.use_var(idx_var);
                let one = builder.ins().iconst(types::I64, 1);
                let next_idx = builder.ins().iadd(current_idx, one);
                builder.def_var(idx_var, next_idx);
            }

            if let Some(elem_ptr_var) = array_elem_ptr_var {
                let elem_ptr = builder.use_var(elem_ptr_var);
                let next_ptr = builder.ins().iadd_imm(elem_ptr, ARRAY_ELEM_SIZE);
                builder.def_var(elem_ptr_var, next_ptr);
            }

            builder.ins().jump(header_block, &[]);
            builder.seal_block(header_block);

            ctx.pop_loop();

            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
        }
    }

    Ok(())
}
