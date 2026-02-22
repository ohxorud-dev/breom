use super::*;

pub(crate) fn execute_defers(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
) -> Result<()> {
    let defers: Vec<DeferBody> = ctx.defer_stack.iter().rev().cloned().collect();

    for defer_body in defers {
        match defer_body {
            DeferBody::Block(block) => {
                for stmt in &block.statements {
                    compile_statement(codegen, builder, ctx, stmt)?;
                }
            }
            DeferBody::Expression(expr_body) => {
                expr::compile_expression(codegen, builder, ctx, &expr_body)?;
            }
        }
    }

    Ok(())
}
