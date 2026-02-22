use super::*;

pub fn compile_default_value(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    var_type: &VarType,
) -> Result<Value> {
    let mut stack = Vec::new();
    compile_default_value_inner(codegen, builder, ctx, var_type, &mut stack, true)
}

pub fn compile_default_value_fieldwise(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    var_type: &VarType,
) -> Result<Value> {
    let mut stack = Vec::new();
    compile_default_value_inner(codegen, builder, ctx, var_type, &mut stack, false)
}

pub(super) fn compile_default_value_inner(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    var_type: &VarType,
    visiting_structs: &mut Vec<String>,
    allow_struct_override: bool,
) -> Result<Value> {
    match var_type {
        VarType::Int | VarType::Bool | VarType::Float => Ok(builder.ins().iconst(types::I64, 0)),
        VarType::String => compile_string_literal(codegen, builder, ""),
        VarType::Struct(struct_name) => {
            let resolved_struct_name = codegen.resolve_struct_type_name(struct_name);
            if visiting_structs.iter().any(|s| s == &resolved_struct_name) {
                return Ok(builder.ins().iconst(types::I64, 0));
            }

            if allow_struct_override {
                let default_func_name = format!("{}__default", resolved_struct_name);
                if codegen.functions.contains_key(&default_func_name) {
                    return compile_call(codegen, builder, ctx, &default_func_name, &[]);
                }
            }

            let type_info = codegen
                .type_registry
                .get(&resolved_struct_name)
                .ok_or_else(|| {
                    anyhow!(
                        "Unknown struct type for default(): {}",
                        resolved_struct_name
                    )
                })?
                .clone();

            let size = builder.ins().iconst(types::I64, type_info.size as i64);
            let type_id = builder.ins().iconst(types::I64, type_info.type_id as i64);
            let ptr = runtime::call_runtime(codegen, builder, "breom_arc_alloc", &[size, type_id])?;

            visiting_structs.push(resolved_struct_name.clone());
            for field in &type_info.fields {
                let field_ty = var_type_from_type_name_with_codegen(codegen, &field.type_name);
                let default_field = compile_default_value_inner(
                    codegen,
                    builder,
                    ctx,
                    &field_ty,
                    visiting_structs,
                    allow_struct_override,
                )?;
                let field_ptr = builder.ins().iadd_imm(ptr, field.offset as i64);
                builder
                    .ins()
                    .store(MemFlags::new(), default_field, field_ptr, 0);
            }
            visiting_structs.pop();

            Ok(ptr)
        }
        VarType::StaticArray(_, _)
        | VarType::DynamicArray(_)
        | VarType::Map(_, _)
        | VarType::Set(_)
        | VarType::Lambda { .. }
        | VarType::Chan(_)
        | VarType::Tuple(_)
        | VarType::Error
        | VarType::Unknown => Ok(builder.ins().iconst(types::I64, 0)),
    }
}
