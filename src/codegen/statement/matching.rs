use super::*;

fn compile_pattern_guard(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    value: Value,
    value_type: &VarType,
    pattern: &Pattern,
    flow: PatternGuardFlow,
) -> Result<()> {
    let PatternGuardFlow {
        success_block,
        fail_block,
    } = flow;

    match pattern {
        Pattern::Wildcard(_) => {
            builder.ins().jump(success_block, &[]);
        }
        Pattern::Binding(var_name, _) => {
            let var = ctx.create_variable(builder, var_name, types::I64);
            builder.def_var(var, value);
            builder.ins().jump(success_block, &[]);
        }
        Pattern::Literal(lit) => {
            let lit_val = literals::compile_literal(codegen, builder, ctx, lit)?;
            let is_match = builder.ins().icmp(IntCC::Equal, value, lit_val);
            builder
                .ins()
                .brif(is_match, success_block, &[], fail_block, &[]);
        }
        Pattern::Enum(enum_pattern) => {
            let enum_name = match value_type {
                VarType::Struct(name) => codegen.resolve_struct_type_name(name),
                _ => {
                    return Err(anyhow!(
                        "Enum pattern '{}' requires enum-typed match expression",
                        enum_pattern.name
                    ));
                }
            };

            let variants = codegen.enum_variants.get(&enum_name).ok_or_else(|| {
                anyhow!(
                    "Enum pattern '{}' used with non-enum type '{}'",
                    enum_pattern.name,
                    enum_name
                )
            })?;
            let variant = variants.get(&enum_pattern.name).ok_or_else(|| {
                anyhow!("Unknown enum variant '{}.{}'", enum_name, enum_pattern.name)
            })?;
            let variant_tag = variant.tag;
            let variant_payload_types = variant.payload_types.clone();
            if variant_payload_types.len() != enum_pattern.patterns.len() {
                return Err(anyhow!(
                    "Enum pattern '{}.{}' expects {} payload(s), got {}",
                    enum_name,
                    enum_pattern.name,
                    variant_payload_types.len(),
                    enum_pattern.patterns.len()
                ));
            }

            let enum_type_info = codegen
                .type_registry
                .get(&enum_name)
                .ok_or_else(|| anyhow!("Unknown enum type: {}", enum_name))?
                .clone();
            let tag_field = enum_type_info.get_field("__tag").ok_or_else(|| {
                anyhow!("Internal enum layout missing tag field for '{}'", enum_name)
            })?;

            let tag_ptr = builder.ins().iadd_imm(value, tag_field.offset as i64);
            let tag_val = builder.ins().load(types::I64, MemFlags::new(), tag_ptr, 0);
            let expected_tag = builder.ins().iconst(types::I64, variant_tag);
            let is_match = builder.ins().icmp(IntCC::Equal, tag_val, expected_tag);
            let payload_block = builder.create_block();
            builder
                .ins()
                .brif(is_match, payload_block, &[], fail_block, &[]);

            builder.switch_to_block(payload_block);
            builder.seal_block(payload_block);

            if enum_pattern.patterns.is_empty() {
                builder.ins().jump(success_block, &[]);
                return Ok(());
            }

            for (idx, payload_pattern) in enum_pattern.patterns.iter().enumerate() {
                let payload_field_name = format!("__payload{}", idx);
                let payload_field =
                    enum_type_info
                        .get_field(&payload_field_name)
                        .ok_or_else(|| {
                            anyhow!(
                                "Internal enum layout missing payload field '{}' for '{}'",
                                payload_field_name,
                                enum_name
                            )
                        })?;

                let payload_ptr = builder.ins().iadd_imm(value, payload_field.offset as i64);
                let payload_val = builder
                    .ins()
                    .load(types::I64, MemFlags::new(), payload_ptr, 0);
                let payload_type = typing::var_type_from_type_name_with_codegen(
                    codegen,
                    &variant_payload_types[idx],
                );

                let next_success = if idx + 1 == enum_pattern.patterns.len() {
                    success_block
                } else {
                    builder.create_block()
                };

                compile_pattern_guard(
                    codegen,
                    builder,
                    ctx,
                    payload_val,
                    &payload_type,
                    payload_pattern,
                    PatternGuardFlow {
                        success_block: next_success,
                        fail_block,
                    },
                )?;

                if idx + 1 < enum_pattern.patterns.len() {
                    builder.switch_to_block(next_success);
                    builder.seal_block(next_success);
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct PatternGuardFlow {
    success_block: Block,
    fail_block: Block,
}

pub(super) fn compile_match(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    match_stmt: &MatchStmt,
) -> Result<()> {
    let match_val = expr::compile_expression(codegen, builder, ctx, &match_stmt.expr)?;
    let match_val_type = typing::infer_expr_type(codegen, ctx, &match_stmt.expr);

    let merge_block = builder.create_block();

    let mut check_blocks: Vec<Block> = Vec::new();
    let mut arm_blocks: Vec<Block> = Vec::new();

    for _ in &match_stmt.arms {
        check_blocks.push(builder.create_block());
        arm_blocks.push(builder.create_block());
    }

    let default_block = builder.create_block();

    if !check_blocks.is_empty() {
        builder.ins().jump(check_blocks[0], &[]);
    } else {
        builder.ins().jump(merge_block, &[]);
    }

    for (i, arm) in match_stmt.arms.iter().enumerate() {
        let check_block = check_blocks[i];
        let arm_block = arm_blocks[i];
        let next_check = if i + 1 < check_blocks.len() {
            check_blocks[i + 1]
        } else {
            default_block
        };

        builder.switch_to_block(check_block);
        builder.seal_block(check_block);

        compile_pattern_guard(
            codegen,
            builder,
            ctx,
            match_val,
            &match_val_type,
            &arm.pattern,
            PatternGuardFlow {
                success_block: arm_block,
                fail_block: next_check,
            },
        )?;
    }

    for (i, arm) in match_stmt.arms.iter().enumerate() {
        let arm_block = arm_blocks[i];

        builder.switch_to_block(arm_block);
        builder.seal_block(arm_block);

        match &arm.body {
            MatchArmBody::Block(block) => {
                for stmt in &block.statements {
                    compile_statement(codegen, builder, ctx, stmt)?;
                }
            }
            MatchArmBody::Expression(expr_body) => {
                expr::compile_expression(codegen, builder, ctx, expr_body)?;
            }
        }

        builder.ins().jump(merge_block, &[]);
    }

    builder.switch_to_block(default_block);
    builder.seal_block(default_block);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(())
}
