use super::*;

pub(super) fn compile_lambda(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    _ctx: &mut FunctionContext,
    lambda: &LambdaExpr,
) -> Result<Value> {
    let lambda_name = format!("__lambda_{}__", codegen.lambda_counter);
    codegen.lambda_counter += 1;

    let mut sig = codegen.module.make_signature();

    for param in &lambda.params {
        let ty = if let Some(ref type_expr) = param.type_annotation {
            codegen.convert_type(type_expr).unwrap_or(types::I64)
        } else {
            types::I64
        };
        sig.params.push(AbiParam::new(ty));
    }

    let return_ty = if let Some(ref type_expr) = lambda.return_type {
        codegen.convert_type(type_expr).unwrap_or(types::I64)
    } else {
        types::I64
    };
    sig.returns.push(AbiParam::new(return_ty));

    let func_id = codegen
        .module
        .declare_function(&lambda_name, Linkage::Local, &sig)
        .map_err(|e| anyhow!("Failed to declare lambda: {}", e))?;

    {
        let mut lambda_ctx = codegen.module.make_context();

        for param in &lambda.params {
            let ty = if let Some(ref type_expr) = param.type_annotation {
                codegen.convert_type(type_expr).unwrap_or(types::I64)
            } else {
                types::I64
            };
            lambda_ctx.func.signature.params.push(AbiParam::new(ty));
        }
        lambda_ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(return_ty));

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut lambda_builder = FunctionBuilder::new(&mut lambda_ctx.func, &mut builder_ctx);

        let entry_block = lambda_builder.create_block();
        lambda_builder.append_block_params_for_function_params(entry_block);
        lambda_builder.switch_to_block(entry_block);

        let mut lambda_func_ctx = FunctionContext::new();

        let params = lambda_builder.block_params(entry_block).to_vec();
        for (i, param) in lambda.params.iter().enumerate() {
            let ty = if let Some(ref type_expr) = param.type_annotation {
                codegen.convert_type(type_expr).unwrap_or(types::I64)
            } else {
                types::I64
            };
            let var = lambda_func_ctx.create_variable(&mut lambda_builder, &param.name, ty);
            lambda_builder.def_var(var, params[i]);
        }

        lambda_builder.seal_block(entry_block);

        let mut needs_implicit_return = true;
        match &lambda.body {
            LambdaBody::Expression(expr) => {
                let result =
                    compile_expression(codegen, &mut lambda_builder, &mut lambda_func_ctx, expr)?;
                lambda_builder.ins().return_(&[result]);
            }
            LambdaBody::Block(block) => {
                let mut last_value = None;
                for stmt in &block.statements {
                    let is_terminal = matches!(stmt, Statement::Return(_) | Statement::Throw(..));
                    last_value = statement::compile_statement(
                        codegen,
                        &mut lambda_builder,
                        &mut lambda_func_ctx,
                        stmt,
                    )?;
                    if is_terminal {
                        needs_implicit_return = false;
                        break;
                    }
                }

                if needs_implicit_return {
                    let result = last_value.unwrap_or_else(|| {
                        if return_ty == types::F64 {
                            lambda_builder.ins().f64const(0.0)
                        } else {
                            lambda_builder.ins().iconst(return_ty, 0)
                        }
                    });
                    lambda_builder.ins().return_(&[result]);
                }
            }
        }

        lambda_builder.finalize();

        codegen
            .module
            .define_function(func_id, &mut lambda_ctx)
            .map_err(|e| anyhow!("Failed to define lambda: {}", e))?;

        codegen.module.clear_context(&mut lambda_ctx);
    }

    codegen.functions.insert(lambda_name.clone(), func_id);

    let func_ref = codegen.module.declare_func_in_func(func_id, builder.func);
    let func_ptr = builder.ins().func_addr(types::I64, func_ref);

    Ok(func_ptr)
}
