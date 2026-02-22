use super::*;

fn compile_wait_arm_body(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    body: &WaitArmBody,
) -> Result<()> {
    match body {
        WaitArmBody::Block(block) => {
            for stmt in &block.statements {
                compile_statement(codegen, builder, ctx, stmt)?;
            }
        }
        WaitArmBody::Expression(expr_body) => {
            expr::compile_expression(codegen, builder, ctx, expr_body)?;
        }
    }

    Ok(())
}

pub(super) fn compile_wait(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    wait_stmt: &WaitStmt,
) -> Result<()> {
    let mut channel_arms: Vec<&WaitArm> = Vec::new();
    let mut default_arm: Option<&WaitArm> = None;
    let mut timeout_arm: Option<(&WaitArm, &Expression)> = None;

    for arm in &wait_stmt.arms {
        match &arm.receiver {
            WaitReceiver::Channel { .. } => channel_arms.push(arm),
            WaitReceiver::Default => {
                if default_arm.is_some() {
                    return Err(anyhow!("wait supports at most one default arm"));
                }
                default_arm = Some(arm);
            }
            WaitReceiver::Timeout(duration) => {
                if timeout_arm.is_some() {
                    return Err(anyhow!("wait supports at most one timeout arm"));
                }
                timeout_arm = Some((arm, duration));
            }
        }
    }

    if default_arm.is_some() && timeout_arm.is_some() {
        return Err(anyhow!("wait cannot use default and timeout arms together"));
    }

    let loop_block = builder.create_block();
    let merge_block = builder.create_block();

    builder.ins().jump(loop_block, &[]);
    builder.switch_to_block(loop_block);

    let select_epoch = runtime::call_runtime(codegen, builder, "breom_select_epoch", &[])?;

    for arm in &channel_arms {
        let (var, channel) = match &arm.receiver {
            WaitReceiver::Channel { var, channel } => (var, channel),
            _ => continue,
        };

        let chan_val = expr::compile_expression(codegen, builder, ctx, channel)?;

        let slot =
            builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 8));
        let slot_addr = builder.ins().stack_addr(types::I64, slot, 0);
        let success = runtime::call_runtime(
            codegen,
            builder,
            "breom_chan_try_recv",
            &[chan_val, slot_addr],
        )?;

        let arm_block = builder.create_block();
        let next_check_block = builder.create_block();

        let zero = builder.ins().iconst(types::I64, 0);
        let is_success = builder.ins().icmp(IntCC::NotEqual, success, zero);
        builder
            .ins()
            .brif(is_success, arm_block, &[], next_check_block, &[]);

        builder.switch_to_block(arm_block);
        builder.seal_block(arm_block);

        let val = builder
            .ins()
            .load(types::I64, MemFlags::new(), slot_addr, 0);
        ctx.enter_scope();
        let v = ctx.create_variable(builder, var, types::I64);
        builder.def_var(v, val);

        let chan_type = typing::infer_expr_type(codegen, ctx, channel);
        let elem_type = match chan_type {
            VarType::Chan(elem) => *elem,
            _ => VarType::Unknown,
        };
        ctx.set_var_type(var, elem_type.clone());
        if elem_type.is_heap_type() {
            ctx.register_heap_var(var);
        }

        compile_wait_arm_body(codegen, builder, ctx, &arm.body)?;
        runtime::release_scope_vars(codegen, builder, ctx)?;
        builder.ins().jump(merge_block, &[]);

        builder.switch_to_block(next_check_block);
        builder.seal_block(next_check_block);
    }

    if let Some(arm) = default_arm {
        ctx.enter_scope();
        compile_wait_arm_body(codegen, builder, ctx, &arm.body)?;
        runtime::release_scope_vars(codegen, builder, ctx)?;
        builder.ins().jump(merge_block, &[]);
    } else if let Some((arm, duration_expr)) = timeout_arm {
        let duration_ty = typing::infer_expr_type(codegen, ctx, duration_expr);
        if !matches!(duration_ty, VarType::Int | VarType::Unknown) {
            return Err(anyhow!(
                "wait timeout duration must be Int-compatible, got {:?}",
                duration_ty
            ));
        }

        let duration = expr::compile_expression(codegen, builder, ctx, duration_expr)?;
        runtime::call_runtime(codegen, builder, "breom_thread_sleep", &[duration])?;
        ctx.enter_scope();
        compile_wait_arm_body(codegen, builder, ctx, &arm.body)?;
        runtime::release_scope_vars(codegen, builder, ctx)?;
        builder.ins().jump(merge_block, &[]);
    } else {
        runtime::call_runtime(codegen, builder, "breom_select_wait", &[select_epoch])?;
        builder.ins().jump(loop_block, &[]);
    }

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);
    builder.seal_block(loop_block);

    Ok(())
}
