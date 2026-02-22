use super::*;

pub(super) fn compile_struct_literal(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    struct_lit: &StructLiteral,
) -> Result<Value> {
    let type_name = match &struct_lit.type_expr {
        TypeExpr::Base(base) => codegen.resolve_struct_type_name(&base.name),
        TypeExpr::Generic(generic) => codegen.generic_type_name(generic),
        _ => return Err(anyhow!("Complex struct types not yet supported")),
    };

    codegen.ensure_instantiated_struct_type(&type_name)?;

    let type_info = codegen
        .type_registry
        .get(&type_name)
        .ok_or_else(|| anyhow!("Unknown struct type: {}", type_name))?
        .clone();

    let size = builder.ins().iconst(types::I64, type_info.size as i64);
    let type_id = builder.ins().iconst(types::I64, type_info.type_id as i64);
    let ptr = runtime::call_runtime(codegen, builder, "breom_arc_alloc", &[size, type_id])?;

    for field_init in &struct_lit.fields {
        let field_info = type_info
            .get_field(&field_init.name)
            .ok_or_else(|| anyhow!("Unknown field: {}", field_init.name))?;

        let value = compile_expression(codegen, builder, ctx, &field_init.value)?;
        let offset = builder.ins().iconst(types::I64, field_info.offset as i64);
        let field_ptr = builder.ins().iadd(ptr, offset);

        builder.ins().store(MemFlags::new(), value, field_ptr, 0);
    }

    Ok(ptr)
}

pub fn compile_new(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    new_expr: &NewExpr,
) -> Result<Value> {
    if new_expr.type_name == "Error" {
        if new_expr.args.len() != 1 {
            return Err(anyhow!("new Error() requires 1 argument (message)"));
        }
        let msg = compile_expression(codegen, builder, ctx, &new_expr.args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_error_new", &[msg]);
    }

    let resolved_type_name = codegen.resolve_struct_type_name(&new_expr.type_name);
    let func_name = format!("{}__new", resolved_type_name);
    if let Some(&func_id) = codegen.functions.get(&func_name) {
        let mut arg_vals = Vec::new();
        let expected_params = codegen.function_param_types.get(&func_name).cloned();
        for (idx, arg) in new_expr.args.iter().enumerate() {
            let expected = expected_params.as_ref().and_then(|params| params.get(idx));
            arg_vals.push(compile_expression_with_type_hint(
                codegen, builder, ctx, arg, expected,
            )?);
        }

        let func_ref = codegen.module.declare_func_in_func(func_id, builder.func);
        let call = builder.ins().call(func_ref, &arg_vals);
        let results = builder.inst_results(call);

        if let Some(TypeExpr::Tuple(tt)) = codegen.function_return_types.get(&func_name) {
            if tt.element_types.len() >= 2 {
                if let TypeExpr::Base(b) = tt.element_types[0].type_expr.as_ref() {
                    if b.name == "Error" && results.len() >= 2 {
                        let err_val = results[0];
                        let value_val = results[1];

                        if ctx.in_result_context {
                            ctx.result_error = Some(err_val);
                            ctx.result_value = Some(value_val);
                            return Ok(value_val);
                        }

                        let zero = builder.ins().iconst(types::I64, 0);
                        let is_err = builder.ins().icmp(IntCC::NotEqual, err_val, zero);

                        let ok_block = builder.create_block();
                        let panic_block = builder.create_block();
                        builder.ins().brif(is_err, panic_block, &[], ok_block, &[]);

                        builder.switch_to_block(panic_block);
                        builder.seal_block(panic_block);
                        runtime::call_runtime(codegen, builder, "breom_panic", &[err_val])?;
                        builder
                            .ins()
                            .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));

                        builder.switch_to_block(ok_block);
                        builder.seal_block(ok_block);
                        return Ok(value_val);
                    }
                }
            }
        }

        Ok(if results.is_empty() {
            builder.ins().iconst(types::I64, 0)
        } else {
            results[0]
        })
    } else {
        Err(anyhow!(
            "Constructor not found for type: {}",
            new_expr.type_name
        ))
    }
}

pub(super) fn compile_channel_new(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    channel_new: &ChannelNewExpr,
) -> Result<Value> {
    let buffer_size = match channel_new.args.len() {
        0 => builder.ins().iconst(types::I64, 0),
        1 => compile_expression(codegen, builder, ctx, &channel_new.args[0])?,
        _ => return Err(anyhow!("Channel<T>.new() takes at most 1 argument")),
    };

    runtime::call_runtime(codegen, builder, "breom_chan_new", &[buffer_size])
}
