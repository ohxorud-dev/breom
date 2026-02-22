use super::*;

pub fn compile_call(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    name: &str,
    args: &[Expression],
) -> Result<Value> {
    if let Some(val) = try_compile_builtin(codegen, builder, ctx, name, args)? {
        return Ok(val);
    }

    let mangled_name = if name.contains('.') {
        name.to_string()
    } else {
        codegen.mangle_name(name)
    };

    if mangled_name != name {
        if let Some(val) = try_compile_builtin(codegen, builder, ctx, &mangled_name, args)? {
            return Ok(val);
        }
    }

    if name == "net.udp.bind" {
        let port = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_net_bind", &[port]);
    }
    if name == "net.udp.send" {
        let mut arg_vals = Vec::new();
        for arg in args {
            arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
        }
        return runtime::call_runtime(codegen, builder, "breom_net_send", &arg_vals);
    }
    if name == "net.tcp.bind" {
        let port = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_net_tcp_bind", &[port]);
    }
    if name == "net.tcp.connect" {
        let mut arg_vals = Vec::new();
        for arg in args {
            arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
        }
        return runtime::call_runtime(codegen, builder, "breom_net_tcp_connect", &arg_vals);
    }
    if name == "net.tcp.send" {
        let mut arg_vals = Vec::new();
        for arg in args {
            arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
        }
        return runtime::call_runtime(codegen, builder, "breom_net_tcp_send", &arg_vals);
    }
    if name == "net.tcp.recv" {
        let socket = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_net_tcp_recv", &[socket]);
    }
    if name == "file.io.read" {
        let path = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_file_read", &[path]);
    }
    if name == "file.io.read_byte_sum" {
        let path = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_file_read_byte_sum", &[path]);
    }
    if name == "file.io.write" {
        let mut arg_vals = Vec::new();
        for arg in args {
            arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
        }
        return runtime::call_runtime(codegen, builder, "breom_file_write", &arg_vals);
    }
    if name == "file.io.append" {
        let mut arg_vals = Vec::new();
        for arg in args {
            arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
        }
        return runtime::call_runtime(codegen, builder, "breom_file_append", &arg_vals);
    }
    if name == "file.io.exists" {
        let path = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_file_exists", &[path]);
    }
    if name == "file.io.remove" {
        let path = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_file_remove", &[path]);
    }
    if name == "file.io.mkdir" {
        let path = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_file_mkdir", &[path]);
    }
    if name == "file.io.reader" {
        let path = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_file_reader_open", &[path]);
    }
    if name == "file.io.scanner" {
        let path = compile_expression(codegen, builder, ctx, &args[0])?;
        return runtime::call_runtime(codegen, builder, "breom_file_scanner_open", &[path]);
    }

    codegen.check_function_access(&mangled_name)?;
    validate_generic_constraints_for_call(codegen, ctx, &mangled_name, args)?;

    let func_id = *codegen
        .functions
        .get(&mangled_name)
        .or_else(|| {
            if name == "main" {
                codegen.functions.get("main")
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("Function not found: {} (tried {})", name, mangled_name))?;

    let mut arg_vals = Vec::new();
    let expected_params = codegen.function_param_types.get(&mangled_name).cloned();
    for (idx, arg) in args.iter().enumerate() {
        let expected = expected_params.as_ref().and_then(|params| params.get(idx));
        arg_vals.push(compile_expression_with_type_hint(
            codegen, builder, ctx, arg, expected,
        )?);
    }

    let func_ref = codegen.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &arg_vals);

    let results = builder.inst_results(call);

    if let Some(TypeExpr::Tuple(tt)) = codegen.function_return_types.get(&mangled_name) {
        if tt.element_types.len() >= 2 {
            if let TypeExpr::Base(b) = tt.element_types[0].type_expr.as_ref() {
                if b.name == "Error" {
                    let err_val = results[0];
                    let value_val = results[1];

                    if ctx.in_result_context {
                        ctx.result_error = Some(err_val);
                        ctx.result_value = Some(value_val);
                        return Ok(value_val);
                    } else {
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
    }

    if results.is_empty() {
        Ok(builder.ins().iconst(types::I64, 0))
    } else {
        Ok(results[0])
    }
}

pub(super) fn validate_generic_constraints_for_call(
    codegen: &CodeGen,
    ctx: &FunctionContext,
    mangled_name: &str,
    args: &[Expression],
) -> Result<()> {
    let Some(generic_params) = codegen.function_generic_params.get(mangled_name) else {
        return Ok(());
    };
    let Some(param_type_exprs) = codegen.function_param_type_exprs.get(mangled_name) else {
        return Ok(());
    };

    let generic_names: HashSet<String> = generic_params.iter().map(|gp| gp.name.clone()).collect();
    let mut bindings: HashMap<String, String> = HashMap::new();

    for (param_ty, arg_expr) in param_type_exprs.iter().zip(args.iter()) {
        let arg_var_type = infer_expr_type(codegen, ctx, arg_expr);
        let Some(arg_type_name) = var_type_to_type_name(codegen, &arg_var_type) else {
            continue;
        };
        bind_generic_type_args(
            codegen,
            &generic_names,
            param_ty,
            &arg_type_name,
            &mut bindings,
        )?;
    }

    for gp in generic_params {
        if gp.constraints.is_empty() {
            continue;
        }

        let Some(bound_arg) = bindings.get(&gp.name) else {
            return Err(anyhow!(
                "Cannot infer generic argument for '{}' in call to '{}'",
                gp.name,
                mangled_name
            ));
        };

        let satisfies = gp
            .constraints
            .iter()
            .any(|constraint| codegen.constraint_matches_arg(constraint, bound_arg));

        if !satisfies {
            let expected = gp
                .constraints
                .iter()
                .map(|c| codegen.type_expr_name(c))
                .collect::<Vec<_>>()
                .join(" | ");
            return Err(anyhow!(
                "Generic argument '{}' does not satisfy constraint '{}' for '{}'",
                bound_arg,
                expected,
                gp.name
            ));
        }
    }

    Ok(())
}

pub(super) fn bind_generic_type_args(
    codegen: &CodeGen,
    generic_names: &HashSet<String>,
    param_ty: &TypeExpr,
    arg_type_name: &str,
    bindings: &mut HashMap<String, String>,
) -> Result<()> {
    match param_ty {
        TypeExpr::Base(base) => {
            if generic_names.contains(&base.name) {
                if let Some(existing) = bindings.get(&base.name) {
                    if existing != arg_type_name {
                        return Err(anyhow!(
                            "Conflicting inferred generic argument for '{}': '{}' vs '{}'",
                            base.name,
                            existing,
                            arg_type_name
                        ));
                    }
                } else {
                    bindings.insert(base.name.clone(), arg_type_name.to_string());
                }
            }
            Ok(())
        }
        TypeExpr::Generic(generic_ty) => {
            let expected_base = codegen.resolve_struct_type_name(&generic_ty.base);
            let Some((arg_base, arg_inner)) = split_type_name_generics(arg_type_name) else {
                return Ok(());
            };
            if expected_base != arg_base {
                return Ok(());
            }
            let arg_parts = crate::codegen::split_generic_args(arg_inner);
            for (type_arg, actual) in generic_ty.type_args.iter().zip(arg_parts.iter()) {
                bind_generic_type_args(
                    codegen,
                    generic_names,
                    &type_arg.type_expr,
                    actual,
                    bindings,
                )?;
            }
            Ok(())
        }
        TypeExpr::Chan(chan) => {
            let Some(inner) = arg_type_name
                .strip_prefix("Channel<")
                .and_then(|s| s.strip_suffix('>'))
            else {
                return Ok(());
            };
            bind_generic_type_args(codegen, generic_names, &chan.element_type, inner, bindings)
        }
        TypeExpr::DynamicArray(arr) => {
            let Some(inner) = arg_type_name.strip_prefix("[]") else {
                return Ok(());
            };
            bind_generic_type_args(codegen, generic_names, &arr.element_type, inner, bindings)
        }
        TypeExpr::Array(arr) => {
            let Some(after_bracket) = arg_type_name.strip_prefix('[') else {
                return Ok(());
            };
            let Some(pos) = after_bracket.find(']') else {
                return Ok(());
            };
            let inner = &after_bracket[pos + 1..];
            bind_generic_type_args(codegen, generic_names, &arr.element_type, inner, bindings)
        }
        _ => Ok(()),
    }
}

pub(super) fn split_type_name_generics(type_name: &str) -> Option<(String, &str)> {
    let lt = type_name.find('<')?;
    let gt = type_name.rfind('>')?;
    if gt <= lt {
        return None;
    }
    Some((type_name[..lt].to_string(), &type_name[lt + 1..gt]))
}

pub(super) fn var_type_to_type_name(codegen: &CodeGen, var_type: &VarType) -> Option<String> {
    match var_type {
        VarType::Int => Some("Int".to_string()),
        VarType::Float => Some("Float".to_string()),
        VarType::Bool => Some("Bool".to_string()),
        VarType::String => Some("String".to_string()),
        VarType::Error => Some("Error".to_string()),
        VarType::Struct(name) => Some(codegen.resolve_struct_type_name(name)),
        VarType::DynamicArray(inner) => {
            var_type_to_type_name(codegen, inner).map(|n| format!("[]{}", n))
        }
        VarType::StaticArray(inner, len) => {
            var_type_to_type_name(codegen, inner).map(|n| format!("[{}]{}", len, n))
        }
        VarType::Chan(inner) => {
            var_type_to_type_name(codegen, inner).map(|n| format!("Channel<{}>", n))
        }
        _ => None,
    }
}

pub(super) fn coerce_print_arg_to_string(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    value: Value,
    var_type: &VarType,
) -> Result<Value> {
    match var_type {
        VarType::String => Ok(value),
        VarType::Int => runtime::call_runtime(codegen, builder, "breom_int_to_string", &[value]),
        VarType::Float => {
            runtime::call_runtime(codegen, builder, "breom_float_to_string", &[value])
        }
        VarType::Struct(_) | VarType::Error => {
            let source_type = match var_type {
                VarType::Struct(name) => name.clone(),
                VarType::Error => "Error".to_string(),
                _ => unreachable!(),
            };
            let has_string_convertable_iface = codegen.interfaces.keys().any(|iface_name| {
                (iface_name == "StringConvertable" || iface_name.ends_with(".StringConvertable"))
                    && codegen.struct_implements_interface(&source_type, iface_name)
            });
            if !has_string_convertable_iface {
                return Err(anyhow!(
                    "print()/println() requires String or StringConvertable. Got {:?}",
                    var_type
                ));
            }

            let Some(func_name) =
                codegen.try_resolve_struct_conversion_name(&source_type, "String")?
            else {
                return Err(anyhow!(
                    "Type '{}' implements StringConvertable but is missing `to String` conversion",
                    source_type
                ));
            };

            compile_struct_method_call(
                codegen,
                builder,
                ctx,
                Some(&source_type),
                value,
                &func_name,
                &[],
            )
        }
        _ => Err(anyhow!(
            "print()/println() requires String or StringConvertable. Got {:?}",
            var_type
        )),
    }
}

pub(super) fn try_compile_builtin(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    name: &str,
    args: &[Expression],
) -> Result<Option<Value>> {
    match name {
        "print" => {
            for arg in args {
                let val = compile_expression(codegen, builder, ctx, arg)?;
                let ty = infer_expr_type(codegen, ctx, arg);
                let str_val = coerce_print_arg_to_string(codegen, builder, ctx, val, &ty)?;
                runtime::call_runtime(codegen, builder, "breom_string_print", &[str_val])?;
            }
            Ok(Some(builder.ins().iconst(types::I64, 0)))
        }
        "println" => {
            for arg in args {
                let val = compile_expression(codegen, builder, ctx, arg)?;
                let ty = infer_expr_type(codegen, ctx, arg);
                let str_val = coerce_print_arg_to_string(codegen, builder, ctx, val, &ty)?;
                runtime::call_runtime(codegen, builder, "breom_string_println", &[str_val])?;
            }
            Ok(Some(builder.ins().iconst(types::I64, 0)))
        }
        "sleep" => {
            if args.is_empty() {
                return Ok(Some(builder.ins().iconst(types::I64, 0)));
            }
            let val = compile_expression(codegen, builder, ctx, &args[0])?;
            runtime::call_runtime(codegen, builder, "breom_thread_sleep", &[val])?;
            Ok(Some(builder.ins().iconst(types::I64, 0)))
        }
        "len" => {
            if args.len() != 1 {
                return Err(anyhow!("len() takes exactly 1 argument"));
            }
            let val = compile_expression(codegen, builder, ctx, &args[0])?;
            let ty = infer_expr_type(codegen, ctx, &args[0]);
            match ty {
                VarType::String => {
                    let res = runtime::call_runtime(codegen, builder, "breom_string_len", &[val])?;
                    Ok(Some(res))
                }
                VarType::DynamicArray(_) | VarType::StaticArray(_, _) => {
                    let res = runtime::call_runtime(codegen, builder, "breom_array_len", &[val])?;
                    Ok(Some(res))
                }
                VarType::Tuple(_) => {
                    let res = runtime::call_runtime(codegen, builder, "breom_array_len", &[val])?;
                    Ok(Some(res))
                }
                VarType::Map(_, _) => {
                    let res = runtime::call_runtime(codegen, builder, "breom_map_len", &[val])?;
                    Ok(Some(res))
                }
                VarType::Set(_) => {
                    let res = runtime::call_runtime(codegen, builder, "breom_set_len", &[val])?;
                    Ok(Some(res))
                }
                _ => Err(anyhow!("len() not supported for {:?}", ty)),
            }
        }
        "string" => Err(anyhow!("string() is removed; use `expr as String`")),
        "net.udp.bind" => {
            if args.len() != 1 {
                return Err(anyhow!("net.udp.bind() takes exactly 1 argument"));
            }
            let val = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_net_bind", &[val])?;
            Ok(Some(res))
        }
        "net.udp.send" => {
            if args.len() != 4 {
                return Err(anyhow!("net.udp.send() takes exactly 4 arguments"));
            }
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
            }
            let res = runtime::call_runtime(codegen, builder, "breom_net_send", &arg_vals)?;
            Ok(Some(res))
        }
        "net.tcp.bind" => {
            if args.len() != 1 {
                return Err(anyhow!("net.tcp.bind() takes exactly 1 argument"));
            }
            let port = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_net_tcp_bind", &[port])?;
            Ok(Some(res))
        }
        "net.tcp.connect" => {
            if args.len() != 2 {
                return Err(anyhow!("net.tcp.connect() takes exactly 2 arguments"));
            }
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
            }
            let res = runtime::call_runtime(codegen, builder, "breom_net_tcp_connect", &arg_vals)?;
            Ok(Some(res))
        }
        "net.tcp.send" => {
            if args.len() != 3 {
                return Err(anyhow!("net.tcp.send() takes exactly 3 arguments"));
            }
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
            }
            let res = runtime::call_runtime(codegen, builder, "breom_net_tcp_send", &arg_vals)?;
            Ok(Some(res))
        }
        "net.tcp.recv" => {
            if args.len() != 1 {
                return Err(anyhow!("net.tcp.recv() takes exactly 1 argument"));
            }
            let socket = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_net_tcp_recv", &[socket])?;
            Ok(Some(res))
        }
        "net.http.parse_status" => {
            if args.len() != 1 {
                return Err(anyhow!("net.http.parse_status() takes exactly 1 argument"));
            }
            let raw = compile_expression(codegen, builder, ctx, &args[0])?;
            let res =
                runtime::call_runtime(codegen, builder, "breom_net_http_response_status", &[raw])?;
            Ok(Some(res))
        }
        "net.http.parse_headers" => {
            if args.len() != 1 {
                return Err(anyhow!("net.http.parse_headers() takes exactly 1 argument"));
            }
            let raw = compile_expression(codegen, builder, ctx, &args[0])?;
            let res =
                runtime::call_runtime(codegen, builder, "breom_net_http_response_headers", &[raw])?;
            Ok(Some(res))
        }
        "net.http.parse_body" => {
            if args.len() != 1 {
                return Err(anyhow!("net.http.parse_body() takes exactly 1 argument"));
            }
            let raw = compile_expression(codegen, builder, ctx, &args[0])?;
            let res =
                runtime::call_runtime(codegen, builder, "breom_net_http_response_body", &[raw])?;
            Ok(Some(res))
        }
        "file.io.read" => {
            if args.len() != 1 {
                return Err(anyhow!("file.io.read() takes exactly 1 argument"));
            }
            let path = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_file_read", &[path])?;
            Ok(Some(res))
        }
        "file.io.read_byte_sum" => {
            if args.len() != 1 {
                return Err(anyhow!("file.io.read_byte_sum() takes exactly 1 argument"));
            }
            let path = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_file_read_byte_sum", &[path])?;
            Ok(Some(res))
        }
        "file.io.write" => {
            if args.len() != 2 {
                return Err(anyhow!("file.io.write() takes exactly 2 arguments"));
            }
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
            }
            let res = runtime::call_runtime(codegen, builder, "breom_file_write", &arg_vals)?;
            Ok(Some(res))
        }
        "file.io.append" => {
            if args.len() != 2 {
                return Err(anyhow!("file.io.append() takes exactly 2 arguments"));
            }
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(compile_expression(codegen, builder, ctx, arg)?);
            }
            let res = runtime::call_runtime(codegen, builder, "breom_file_append", &arg_vals)?;
            Ok(Some(res))
        }
        "file.io.exists" => {
            if args.len() != 1 {
                return Err(anyhow!("file.io.exists() takes exactly 1 argument"));
            }
            let path = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_file_exists", &[path])?;
            Ok(Some(res))
        }
        "file.io.remove" => {
            if args.len() != 1 {
                return Err(anyhow!("file.io.remove() takes exactly 1 argument"));
            }
            let path = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_file_remove", &[path])?;
            Ok(Some(res))
        }
        "file.io.mkdir" => {
            if args.len() != 1 {
                return Err(anyhow!("file.io.mkdir() takes exactly 1 argument"));
            }
            let path = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_file_mkdir", &[path])?;
            Ok(Some(res))
        }
        "file.io.reader" => {
            if args.len() != 1 {
                return Err(anyhow!("file.io.reader() takes exactly 1 argument"));
            }
            let path = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_file_reader_open", &[path])?;
            Ok(Some(res))
        }
        "file.io.scanner" => {
            if args.len() != 1 {
                return Err(anyhow!("file.io.scanner() takes exactly 1 argument"));
            }
            let path = compile_expression(codegen, builder, ctx, &args[0])?;
            let res = runtime::call_runtime(codegen, builder, "breom_file_scanner_open", &[path])?;
            Ok(Some(res))
        }
        "assert" => {
            if !codegen.is_test_mode() {
                return Ok(None);
            }
            let mangled = codegen.mangle_name("assert");
            if codegen.functions.contains_key(&mangled) || codegen.functions.contains_key("assert")
            {
                return Ok(None);
            }
            if args.len() != 1 {
                return Err(anyhow!("assert() takes exactly 1 argument"));
            }
            let cond = compile_expression(codegen, builder, ctx, &args[0])?;
            runtime::call_runtime(codegen, builder, "breom_test_assert", &[cond])?;
            Ok(Some(builder.ins().iconst(types::I64, 0)))
        }
        "fail" => {
            if !codegen.is_test_mode() {
                return Ok(None);
            }
            let mangled = codegen.mangle_name("fail");
            if codegen.functions.contains_key(&mangled) || codegen.functions.contains_key("fail") {
                return Ok(None);
            }
            if args.len() != 1 {
                return Err(anyhow!("fail() takes exactly 1 argument"));
            }
            let message = compile_expression(codegen, builder, ctx, &args[0])?;
            runtime::call_runtime(codegen, builder, "breom_test_fail", &[message])?;
            Ok(Some(builder.ins().iconst(types::I64, 0)))
        }
        _ => Ok(None),
    }
}
