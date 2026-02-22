use super::*;

pub(super) fn compile_spawn(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    spawn_stmt: &SpawnStmt,
) -> Result<()> {
    let captured_vars: Vec<(String, VarType)> = ctx
        .var_types
        .iter()
        .map(|(name, ty)| (name.clone(), ty.clone()))
        .collect();

    let env_size = (captured_vars.len() * 8) as i64;
    let env_size_val = builder.ins().iconst(types::I64, env_size);
    let type_id_val = builder.ins().iconst(types::I64, 6);
    let env_ptr = runtime::call_runtime(
        codegen,
        builder,
        "breom_arc_alloc",
        &[env_size_val, type_id_val],
    )?;

    for (i, (name, ty)) in captured_vars.iter().enumerate() {
        let val = ctx.get_variable(builder, name)?;
        if ty.is_heap_type() {
            runtime::call_runtime(codegen, builder, "breom_arc_retain", &[val])?;
        }
        let offset = (i * 8) as i32;
        builder.ins().store(MemFlags::new(), val, env_ptr, offset);
    }

    let wrapper_name = format!("__spawn_wrapper_{}__", codegen.lambda_counter);
    codegen.lambda_counter += 1;

    let mut sig = codegen.module.make_signature();
    sig.params
        .push(cranelift_codegen::ir::AbiParam::new(types::I64));

    let func_id = codegen
        .module
        .declare_function(&wrapper_name, Linkage::Local, &sig)
        .map_err(|e| anyhow!("Failed to declare spawn wrapper: {}", e))?;

    let mut wrapper_ctx = codegen.module.make_context();
    wrapper_ctx.func.signature = sig.clone();

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut wrapper_builder = FunctionBuilder::new(&mut wrapper_ctx.func, &mut builder_ctx);

    let entry_block = wrapper_builder.create_block();
    wrapper_builder.append_block_params_for_function_params(entry_block);
    wrapper_builder.switch_to_block(entry_block);
    wrapper_builder.seal_block(entry_block);

    let env_ptr_val = wrapper_builder.block_params(entry_block)[0];
    let mut wrapper_func_ctx = FunctionContext::new();

    for (i, (name, ty)) in captured_vars.iter().enumerate() {
        let offset = (i * 8) as i32;
        let val = wrapper_builder
            .ins()
            .load(types::I64, MemFlags::new(), env_ptr_val, offset);
        let var = wrapper_func_ctx.create_variable(&mut wrapper_builder, name, types::I64);
        wrapper_builder.def_var(var, val);
        wrapper_func_ctx.var_types.insert(name.clone(), ty.clone());
        if ty.is_heap_type() {
            wrapper_func_ctx.heap_vars.push(name.clone());
        }
    }

    match &spawn_stmt.body {
        SpawnBody::Block(block) => {
            for stmt in &block.statements {
                compile_statement(codegen, &mut wrapper_builder, &mut wrapper_func_ctx, stmt)?;
            }
        }
        SpawnBody::Expression(expr_body) => {
            expr::compile_expression(
                codegen,
                &mut wrapper_builder,
                &mut wrapper_func_ctx,
                expr_body,
            )?;
        }
    }

    runtime::release_scope_vars(codegen, &mut wrapper_builder, &mut wrapper_func_ctx)?;

    wrapper_builder.ins().return_(&[]);
    wrapper_builder.finalize();

    codegen
        .module
        .define_function(func_id, &mut wrapper_ctx)
        .map_err(|e| anyhow!("Failed to define spawn wrapper: {}", e))?;
    codegen.module.clear_context(&mut wrapper_ctx);

    codegen.functions.insert(wrapper_name.clone(), func_id);

    let func_ref = codegen.module.declare_func_in_func(func_id, builder.func);
    let func_ptr = builder.ins().func_addr(types::I64, func_ref);
    runtime::call_runtime(codegen, builder, "breom_spawn", &[func_ptr, env_ptr])?;

    Ok(())
}
