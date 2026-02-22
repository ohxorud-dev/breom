use super::*;

pub(super) fn compile_collection(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    coll: &CollectionLiteral,
) -> Result<Value> {
    match coll {
        CollectionLiteral::DynamicArray(elements, _) => compile_dynamic_array_from_elements(
            codegen,
            builder,
            ctx,
            elements,
            elements.len().max(4),
        ),

        CollectionLiteral::RepeatedArray { value, count, .. } => {
            compile_repeated_array(codegen, builder, ctx, value, count)
        }

        CollectionLiteral::Map(entries, _) => {
            let initial_cap = builder
                .ins()
                .iconst(types::I64, entries.len().max(8) as i64);
            let map_ptr = runtime::call_runtime(codegen, builder, "breom_map_new", &[initial_cap])?;

            for (key_expr, value_expr) in entries {
                let key = compile_expression(codegen, builder, ctx, key_expr)?;
                let value = compile_expression(codegen, builder, ctx, value_expr)?;
                runtime::call_runtime(codegen, builder, "breom_map_set", &[map_ptr, key, value])?;
            }

            Ok(map_ptr)
        }

        CollectionLiteral::Set(elements, _) => {
            let initial_cap = builder
                .ins()
                .iconst(types::I64, elements.len().max(8) as i64);
            let set_ptr = runtime::call_runtime(codegen, builder, "breom_set_new", &[initial_cap])?;

            for elem in elements {
                let value = compile_expression(codegen, builder, ctx, elem)?;
                runtime::call_runtime(codegen, builder, "breom_set_add", &[set_ptr, value])?;
            }

            Ok(set_ptr)
        }
    }
}

pub(super) fn compile_dynamic_array_from_elements(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    elements: &[Expression],
    initial_capacity: usize,
) -> Result<Value> {
    let elem_size = builder.ins().iconst(types::I64, 8);
    let initial_cap = builder
        .ins()
        .iconst(types::I64, initial_capacity.max(1) as i64);
    let mut arr_ptr = runtime::call_runtime(
        codegen,
        builder,
        "breom_array_new",
        &[elem_size, initial_cap],
    )?;

    for elem in elements {
        let value = compile_expression(codegen, builder, ctx, elem)?;
        arr_ptr = runtime::call_runtime(codegen, builder, "breom_array_push", &[arr_ptr, value])?;
    }

    Ok(arr_ptr)
}

pub(super) fn compile_repeated_array(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    value: &Expression,
    count: &Expression,
) -> Result<Value> {
    let count_type = infer_expr_type(codegen, ctx, count);
    if !matches!(count_type, VarType::Int | VarType::Unknown) {
        return Err(anyhow!(
            "Repeat array count must be Int-compatible, got {:?}",
            count_type
        ));
    }

    let count_val = compile_expression(codegen, builder, ctx, count)?;
    let repeated_value = compile_expression(codegen, builder, ctx, value)?;

    let elem_size = builder.ins().iconst(types::I64, 8);
    let initial_cap = builder.ins().iconst(types::I64, 4);
    let arr_ptr = runtime::call_runtime(
        codegen,
        builder,
        "breom_array_new",
        &[elem_size, initial_cap],
    )?;

    let arr_var = builder.declare_var(types::I64);
    let idx_var = builder.declare_var(types::I64);
    let count_var = builder.declare_var(types::I64);
    builder.def_var(arr_var, arr_ptr);
    let zero = builder.ins().iconst(types::I64, 0);
    builder.def_var(idx_var, zero);
    builder.def_var(count_var, count_val);

    let header_block = builder.create_block();
    let body_block = builder.create_block();
    let exit_block = builder.create_block();

    builder.ins().jump(header_block, &[]);

    builder.switch_to_block(header_block);
    let idx = builder.use_var(idx_var);
    let count_now = builder.use_var(count_var);
    let cond = builder.ins().icmp(IntCC::SignedLessThan, idx, count_now);
    builder.ins().brif(cond, body_block, &[], exit_block, &[]);

    builder.switch_to_block(body_block);
    let current_arr = builder.use_var(arr_var);
    let next_arr = runtime::call_runtime(
        codegen,
        builder,
        "breom_array_push",
        &[current_arr, repeated_value],
    )?;
    builder.def_var(arr_var, next_arr);

    let one = builder.ins().iconst(types::I64, 1);
    let next_idx = builder.ins().iadd(idx, one);
    builder.def_var(idx_var, next_idx);
    builder.ins().jump(header_block, &[]);
    builder.seal_block(body_block);
    builder.seal_block(header_block);

    builder.switch_to_block(exit_block);
    builder.seal_block(exit_block);

    Ok(builder.use_var(arr_var))
}

pub fn compile_collection_for_static_array(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    coll: &CollectionLiteral,
    element_type: &VarType,
    declared_len: usize,
) -> Result<Value> {
    let slot_size = (declared_len.max(1) * 8) as u32;
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        slot_size,
        8,
    ));
    let slot_addr = builder.ins().stack_addr(types::I64, slot, 0);

    let initialized = match coll {
        CollectionLiteral::DynamicArray(elements, _) => {
            if elements.len() > declared_len {
                return Err(anyhow!(
                    "Static array length mismatch: declared {}, got {}",
                    declared_len,
                    elements.len()
                ));
            }

            for (idx, elem) in elements.iter().enumerate() {
                let value = compile_expression(codegen, builder, ctx, elem)?;
                let elem_ptr = builder.ins().iadd_imm(slot_addr, (idx as i64) * 8);
                builder.ins().store(MemFlags::new(), value, elem_ptr, 0);
            }
            elements.len()
        }
        CollectionLiteral::RepeatedArray { value, count, .. } => {
            let repeat_count = const_non_negative_int(count).ok_or_else(|| {
                anyhow!(
                    "Static array repeat initializer requires non-negative integer literal count"
                )
            })?;

            if repeat_count > declared_len {
                return Err(anyhow!(
                    "Static array length mismatch: declared {}, got {}",
                    declared_len,
                    repeat_count
                ));
            }

            let repeated_value = compile_expression(codegen, builder, ctx, value)?;
            for idx in 0..repeat_count {
                let elem_ptr = builder.ins().iadd_imm(slot_addr, (idx as i64) * 8);
                builder
                    .ins()
                    .store(MemFlags::new(), repeated_value, elem_ptr, 0);
            }
            repeat_count
        }
        _ => {
            return Err(anyhow!(
                "Static array initializer expects array literal, got {:?}",
                coll
            ));
        }
    };

    for idx in initialized..declared_len {
        let default_value = compile_default_value(codegen, builder, ctx, element_type)?;
        let elem_ptr = builder.ins().iadd_imm(slot_addr, (idx as i64) * 8);
        builder
            .ins()
            .store(MemFlags::new(), default_value, elem_ptr, 0);
    }

    let len_val = builder.ins().iconst(types::I64, declared_len as i64);
    runtime::call_runtime(
        codegen,
        builder,
        "breom_array_from_i64_buffer",
        &[slot_addr, len_val],
    )
}

pub(super) fn compile_tuple(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    tuple: &TupleLiteral,
) -> Result<Value> {
    let elem_size = builder.ins().iconst(types::I64, 8);
    let initial_cap = builder
        .ins()
        .iconst(types::I64, tuple.elements.len().max(2) as i64);
    let mut arr_ptr = runtime::call_runtime(
        codegen,
        builder,
        "breom_array_new",
        &[elem_size, initial_cap],
    )?;

    for elem in &tuple.elements {
        let value = compile_expression(codegen, builder, ctx, elem)?;
        arr_ptr = runtime::call_runtime(codegen, builder, "breom_array_push", &[arr_ptr, value])?;
    }

    Ok(arr_ptr)
}
