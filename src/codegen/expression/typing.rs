use super::*;

pub(crate) fn var_type_from_type_name_with_codegen(codegen: &CodeGen, type_name: &str) -> VarType {
    if let Some(inner) = type_name
        .strip_prefix("Channel<")
        .and_then(|s| s.strip_suffix('>'))
    {
        return VarType::Chan(Box::new(var_type_from_type_name_with_codegen(
            codegen, inner,
        )));
    }
    if let Some(inner) = type_name.strip_prefix("[]") {
        return VarType::DynamicArray(Box::new(var_type_from_type_name_with_codegen(
            codegen, inner,
        )));
    }
    if let Some(inner) = type_name.strip_prefix('[') {
        if let Some(pos) = inner.find(']') {
            let len = inner[..pos].parse::<usize>().unwrap_or(0);
            let elem = &inner[pos + 1..];
            return VarType::StaticArray(
                Box::new(var_type_from_type_name_with_codegen(codegen, elem)),
                len,
            );
        }
    }
    if let Some(base) = type_name
        .strip_prefix("Map<")
        .and_then(|s| s.strip_suffix('>'))
    {
        let args = crate::codegen::split_generic_args(base);
        if args.len() == 2 {
            return VarType::Map(
                Box::new(var_type_from_type_name_with_codegen(codegen, &args[0])),
                Box::new(var_type_from_type_name_with_codegen(codegen, &args[1])),
            );
        }
    }
    if let Some(base) = type_name
        .strip_prefix("Set<")
        .and_then(|s| s.strip_suffix('>'))
    {
        let args = crate::codegen::split_generic_args(base);
        if args.len() == 1 {
            return VarType::Set(Box::new(var_type_from_type_name_with_codegen(
                codegen, &args[0],
            )));
        }
    }
    match type_name {
        "Int" | "Int8" | "Int16" | "Int32" | "Int64" | "UInt" | "UInt8" | "UInt16" | "UInt32"
        | "UInt64" | "Byte" => VarType::Int,
        "Float" | "Float32" | "Float64" => VarType::Float,
        "Bool" => VarType::Bool,
        "String" => VarType::String,
        "Error" => VarType::Error,
        other if other.contains('.') => VarType::Struct(other.to_string()),
        other
            if other
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false) =>
        {
            VarType::Struct(codegen.resolve_struct_type_name(other))
        }
        _ => VarType::Unknown,
    }
}

pub(crate) fn infer_type_expr_to_var_type_with_codegen(
    codegen: &CodeGen,
    type_expr: &TypeExpr,
) -> VarType {
    match type_expr {
        TypeExpr::Base(base) => match base.name.as_str() {
            "Int" | "Int8" | "Int16" | "Int32" | "Int64" | "UInt" | "UInt8" | "UInt16"
            | "UInt32" | "UInt64" | "Byte" => VarType::Int,
            "Float" | "Float32" | "Float64" => VarType::Float,
            "Bool" => VarType::Bool,
            "String" => VarType::String,
            "Error" => VarType::Error,
            _ => VarType::Struct(codegen.resolve_struct_type_name(&base.name)),
        },
        TypeExpr::Chan(chan) => VarType::Chan(Box::new(infer_type_expr_to_var_type_with_codegen(
            codegen,
            &chan.element_type,
        ))),
        TypeExpr::Array(array) => VarType::StaticArray(
            Box::new(infer_type_expr_to_var_type_with_codegen(
                codegen,
                &array.element_type,
            )),
            array.size as usize,
        ),
        TypeExpr::DynamicArray(array) => VarType::DynamicArray(Box::new(
            infer_type_expr_to_var_type_with_codegen(codegen, &array.element_type),
        )),
        TypeExpr::Function(func) => {
            let params = func
                .param_types
                .iter()
                .map(|t| infer_type_expr_to_var_type_with_codegen(codegen, t))
                .collect();
            let return_type = func
                .return_type
                .as_ref()
                .map(|t| infer_type_expr_to_var_type_with_codegen(codegen, t))
                .unwrap_or(VarType::Unknown);
            VarType::Lambda {
                params,
                return_type: Box::new(return_type),
            }
        }
        TypeExpr::Tuple(tuple) => {
            let element_types = tuple
                .element_types
                .iter()
                .map(|tc| infer_type_expr_to_var_type_with_codegen(codegen, &tc.type_expr))
                .collect();
            VarType::Tuple(element_types)
        }
        TypeExpr::Generic(generic) => VarType::Struct(codegen.generic_type_name(generic)),
    }
}

pub(crate) fn infer_type_expr_to_var_type(type_expr: &TypeExpr) -> VarType {
    match type_expr {
        TypeExpr::Base(base) => match base.name.as_str() {
            "Int" | "Int8" | "Int16" | "Int32" | "Int64" | "UInt" | "UInt8" | "UInt16"
            | "UInt32" | "UInt64" | "Byte" => VarType::Int,
            "Float" | "Float32" | "Float64" => VarType::Float,
            "Bool" => VarType::Bool,
            "String" => VarType::String,
            "Error" => VarType::Error,
            _ => VarType::Struct(base.name.clone()),
        },
        TypeExpr::Chan(chan) => {
            VarType::Chan(Box::new(infer_type_expr_to_var_type(&chan.element_type)))
        }
        TypeExpr::Array(array) => VarType::StaticArray(
            Box::new(infer_type_expr_to_var_type(&array.element_type)),
            array.size as usize,
        ),
        TypeExpr::DynamicArray(array) => {
            VarType::DynamicArray(Box::new(infer_type_expr_to_var_type(&array.element_type)))
        }
        TypeExpr::Function(func) => {
            let params = func
                .param_types
                .iter()
                .map(infer_type_expr_to_var_type)
                .collect();
            let return_type = func
                .return_type
                .as_ref()
                .map(|t| infer_type_expr_to_var_type(t))
                .unwrap_or(VarType::Unknown);
            VarType::Lambda {
                params,
                return_type: Box::new(return_type),
            }
        }
        TypeExpr::Tuple(tuple) => {
            let element_types = tuple
                .element_types
                .iter()
                .map(|tc| infer_type_expr_to_var_type(&tc.type_expr))
                .collect();
            VarType::Tuple(element_types)
        }
        TypeExpr::Generic(generic) => VarType::Struct(generic.base.clone()),
    }
}

pub(super) fn clif_type_from_var_type(var_type: &VarType) -> types::Type {
    match var_type {
        VarType::Float => types::F64,
        _ => types::I64,
    }
}

pub(super) fn const_non_negative_int(expr: &Expression) -> Option<usize> {
    if let Expression::Literal(Literal::Integer(v, _)) = expr {
        if *v >= 0 {
            return Some(*v as usize);
        }
    }
    None
}

pub fn infer_expr_type(codegen: &CodeGen, ctx: &FunctionContext, expr: &Expression) -> VarType {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Integer(..) => VarType::Int,
            Literal::Float(..) => VarType::Float,
            Literal::Bool(..) => VarType::Bool,
            Literal::String(..) => VarType::String,
            Literal::MultilineString(..) => VarType::String,
            Literal::Char(..) => VarType::Int,
            Literal::FString(..) => VarType::String,
            _ => VarType::Unknown,
        },
        Expression::Identifier(name, _) => {
            if let Some(define) = codegen.defines.get(name) {
                match define {
                    DefineValue::Int(_) => VarType::Int,
                    DefineValue::Float(_) => VarType::Float,
                    DefineValue::Bool(_) => VarType::Bool,
                    DefineValue::String(_) => VarType::String,
                    DefineValue::Error(_) => VarType::Error,
                }
            } else {
                let ty = ctx.get_var_type(name);
                if matches!(ty, VarType::Unknown) {
                    let mangled = codegen.mangle_name(name);
                    if let Some(global_ty) = codegen.global_var_types.get(&mangled) {
                        return global_ty.clone();
                    }
                }
                ty
            }
        }
        Expression::Binary(bin) => infer_expr_type(codegen, ctx, &bin.left),
        Expression::Grouped(inner, _) => infer_expr_type(codegen, ctx, inner),
        Expression::StructLiteral(sl) => match &sl.type_expr {
            TypeExpr::Base(base) => VarType::Struct(codegen.resolve_struct_type_name(&base.name)),
            TypeExpr::Generic(generic) => VarType::Struct(codegen.generic_type_name(generic)),
            _ => VarType::Unknown,
        },
        Expression::Collection(col) => match col {
            CollectionLiteral::DynamicArray(elems, _) => {
                if elems.is_empty() {
                    VarType::DynamicArray(Box::new(VarType::Unknown))
                } else {
                    let elem_type = infer_expr_type(codegen, ctx, &elems[0]);
                    VarType::DynamicArray(Box::new(elem_type))
                }
            }
            CollectionLiteral::RepeatedArray { value, .. } => {
                let elem_type = infer_expr_type(codegen, ctx, value);
                VarType::DynamicArray(Box::new(elem_type))
            }
            CollectionLiteral::Map(entries, _) => {
                if entries.is_empty() {
                    VarType::Map(Box::new(VarType::Unknown), Box::new(VarType::Unknown))
                } else {
                    let (k, v) = &entries[0];
                    let k_type = infer_expr_type(codegen, ctx, k);
                    let v_type = infer_expr_type(codegen, ctx, v);
                    VarType::Map(Box::new(k_type), Box::new(v_type))
                }
            }
            CollectionLiteral::Set(elems, _) => {
                if elems.is_empty() {
                    VarType::Set(Box::new(VarType::Unknown))
                } else {
                    let elem_type = infer_expr_type(codegen, ctx, &elems[0]);
                    VarType::Set(Box::new(elem_type))
                }
            }
        },
        Expression::TupleLiteral(tuple) => {
            let element_types = tuple
                .elements
                .iter()
                .map(|elem| infer_expr_type(codegen, ctx, elem))
                .collect();
            VarType::Tuple(element_types)
        }
        Expression::Lambda(lambda) => {
            let params = lambda
                .params
                .iter()
                .map(|param| {
                    param
                        .type_annotation
                        .as_ref()
                        .map(|ty| infer_type_expr_to_var_type_with_codegen(codegen, ty))
                        .unwrap_or(VarType::Int)
                })
                .collect();

            let return_type = lambda
                .return_type
                .as_ref()
                .map(|ty| infer_type_expr_to_var_type_with_codegen(codegen, ty))
                .unwrap_or(VarType::Int);

            VarType::Lambda {
                params,
                return_type: Box::new(return_type),
            }
        }
        Expression::Postfix(postfix) => {
            let mut inferred = infer_expr_type(codegen, ctx, &postfix.base);
            let mut i = 0;
            while i < postfix.ops.len() {
                if i + 1 < postfix.ops.len() {
                    if let (PostfixOp::Member(method_name), PostfixOp::Call(args)) =
                        (&postfix.ops[i], &postfix.ops[i + 1])
                    {
                        if let Some(next_ty) =
                            infer_builtin_method_result_type(&inferred, method_name, args.len())
                        {
                            inferred = next_ty;
                            i += 2;
                            continue;
                        }

                        if let VarType::Struct(struct_name) = &inferred {
                            let Some(func_name) =
                                codegen.resolve_struct_method_name(struct_name, method_name)
                            else {
                                let resolved_struct_name =
                                    codegen.resolve_struct_type_name(struct_name);
                                let Ok(Some((
                                    _point_offset,
                                    promoted_func_name,
                                    _point_field_name,
                                    _promoted_struct_name,
                                ))) = resolve_promoted_point_method(
                                    codegen,
                                    &resolved_struct_name,
                                    method_name,
                                )
                                else {
                                    continue;
                                };
                                if let Some(next_ty) =
                                    codegen.function_value_types.get(&promoted_func_name)
                                {
                                    inferred = next_ty.clone();
                                    i += 2;
                                    continue;
                                }
                                continue;
                            };
                            if let Some(next_ty) = codegen.function_value_types.get(&func_name) {
                                inferred = next_ty.clone();
                                i += 2;
                                continue;
                            }
                        }
                    }
                }

                inferred = infer_postfix_result_type(codegen, &inferred, &postfix.ops[i]);
                i += 1;
            }
            if !matches!(inferred, VarType::Unknown) {
                return inferred;
            }

            if let Expression::Identifier(name, _) = postfix.base.as_ref() {
                if !ctx.variables.contains_key(name) {
                    let resolved_name = codegen.resolve_struct_type_name(name);
                    if codegen.type_registry.get(&resolved_name).is_some() {
                        if let (Some(PostfixOp::Member(method_name)), Some(PostfixOp::Call(_))) =
                            (postfix.ops.first(), postfix.ops.get(1))
                        {
                            if method_name == "new" || method_name == "default" {
                                let mut target_type = VarType::Struct(resolved_name);
                                for op in postfix.ops.iter().skip(2) {
                                    target_type =
                                        infer_postfix_result_type(codegen, &target_type, op);
                                }
                                if !matches!(target_type, VarType::Unknown) {
                                    return target_type;
                                }
                            }
                        }
                    }

                    if let (Some(PostfixOp::Member(method_name)), Some(PostfixOp::Call(_))) =
                        (postfix.ops.first(), postfix.ops.get(1))
                    {
                        if let Some(full_pkg) = codegen.current_imports.get(name) {
                            let target_func = format!("{}.{}", full_pkg, method_name);
                            if let Some(mut target_type) =
                                codegen.function_value_types.get(&target_func).cloned()
                            {
                                for op in postfix.ops.iter().skip(2) {
                                    target_type =
                                        infer_postfix_result_type(codegen, &target_type, op);
                                }
                                if !matches!(target_type, VarType::Unknown) {
                                    return target_type;
                                }
                            }
                        }
                    }
                }
            }

            if let Expression::Identifier(name, _) = postfix.base.as_ref() {
                if name == "Chan" {
                    return VarType::Chan(Box::new(VarType::Unknown));
                }
                if name == "error" {
                    return VarType::Error;
                }
                if name == "udp" {
                    if let Some(PostfixOp::Member(m)) = postfix.ops.first() {
                        if m == "bind" {
                            return VarType::Struct(
                                codegen.resolve_struct_type_name("udp.BindResult"),
                            );
                        }
                        if m == "send" {
                            return VarType::Int;
                        }
                    }
                } else if name == "tcp" {
                    if let Some(PostfixOp::Member(m)) = postfix.ops.first() {
                        if m == "bind" {
                            return VarType::Struct(
                                codegen.resolve_struct_type_name("tcp.BindResult"),
                            );
                        }
                        if m == "connect" {
                            return VarType::Int;
                        }
                        if m == "send" {
                            return VarType::Int;
                        }
                        if m == "recv" {
                            return VarType::String;
                        }
                    }
                } else if name == "http" {
                    if let Some(PostfixOp::Member(m)) = postfix.ops.first() {
                        if m == "listen" {
                            return VarType::Unknown;
                        }
                    }
                } else if let Some(PostfixOp::Member(m)) = postfix.ops.last() {
                    if m == "socket" {
                        return VarType::Int;
                    }
                }
            }
            VarType::Unknown
        }
        Expression::ChannelReceive(expr, _) => match infer_expr_type(codegen, ctx, expr) {
            VarType::Chan(elem) => *elem,
            _ => VarType::Unknown,
        },
        Expression::ChannelNew(channel_new) => VarType::Chan(Box::new(
            infer_type_expr_to_var_type_with_codegen(codegen, &channel_new.element_type),
        )),
        Expression::New(new_expr) => {
            if new_expr.type_name == "Error" {
                VarType::Error
            } else {
                VarType::Struct(codegen.resolve_struct_type_name(&new_expr.type_name))
            }
        }
        _ => VarType::Unknown,
    }
}
