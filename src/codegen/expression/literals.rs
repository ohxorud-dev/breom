use super::*;

pub fn compile_literal(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    lit: &Literal,
) -> Result<Value> {
    match lit {
        Literal::Integer(val, _) => Ok(builder.ins().iconst(types::I64, *val)),
        Literal::Float(val, _) => Ok(builder.ins().f64const(*val)),
        Literal::Bool(val, _) => Ok(builder.ins().iconst(types::I64, if *val { 1 } else { 0 })),
        Literal::String(val, _) => compile_string_literal(codegen, builder, val),
        Literal::MultilineString(val, _) => compile_string_literal(codegen, builder, val),
        Literal::Char(c, _) => Ok(builder.ins().iconst(types::I64, *c as i64)),
        Literal::FString(f) => compile_fstring_literal(codegen, builder, ctx, f),
        Literal::Void(_) => Ok(builder.ins().iconst(types::I64, 0)),
    }
}

pub(super) fn compile_fstring_literal(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    f: &FStringLiteral,
) -> Result<Value> {
    let mut current_str = compile_string_literal(codegen, builder, "")?;

    for part in &f.parts {
        let part_val = match part {
            FStringPart::Text(s) => compile_string_literal(codegen, builder, s)?,
            FStringPart::Interpolation(expr) => {
                let val = compile_expression(codegen, builder, ctx, expr)?;
                let ty = infer_expr_type(codegen, ctx, expr);
                match ty {
                    VarType::Int => {
                        runtime::call_runtime(codegen, builder, "breom_int_to_string", &[val])?
                    }
                    VarType::Float => {
                        runtime::call_runtime(codegen, builder, "breom_float_to_string", &[val])?
                    }
                    VarType::String => val,
                    _ => runtime::call_runtime(codegen, builder, "breom_int_to_string", &[val])?,
                }
            }
        };
        current_str = runtime::call_runtime(
            codegen,
            builder,
            "breom_string_concat",
            &[current_str, part_val],
        )?;
    }

    Ok(current_str)
}

pub fn compile_string_literal(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    s: &str,
) -> Result<Value> {
    let data_id = runtime::intern_string(codegen, s)?;
    let data_val = codegen.module.declare_data_in_func(data_id, builder.func);
    let data_ptr = builder.ins().global_value(types::I64, data_val);
    let len_val = builder.ins().iconst(types::I64, s.len() as i64);

    runtime::call_runtime(codegen, builder, "breom_string_new", &[data_ptr, len_val])
}
