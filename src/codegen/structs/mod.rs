use anyhow::{anyhow, Result};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use std::collections::HashMap;

use crate::ast::{common::*, declarations::*, expressions::*, statements::*, types::*};
use crate::codegen::context::FunctionContext;
use crate::codegen::statement;
use crate::codegen::types::VarType;
use crate::codegen::{wrap_return_type, CodeGen};

type ResolutionRules = Vec<(String, String)>;
type StructResolutionAttributes = (ResolutionRules, ResolutionRules);

fn field_type_name(codegen: &CodeGen, type_expr: &TypeExpr, generic_params: &[String]) -> String {
    match type_expr {
        TypeExpr::Base(base) => {
            if generic_params.iter().any(|p| p == &base.name) {
                base.name.clone()
            } else {
                codegen.resolve_struct_type_name(&base.name)
            }
        }
        TypeExpr::Chan(chan) => {
            format!(
                "Channel<{}>",
                field_type_name(codegen, &chan.element_type, generic_params)
            )
        }
        TypeExpr::DynamicArray(arr) => format!(
            "[]{}",
            field_type_name(codegen, &arr.element_type, generic_params)
        ),
        TypeExpr::Array(arr) => {
            format!(
                "[{}]{}",
                arr.size,
                field_type_name(codegen, &arr.element_type, generic_params)
            )
        }
        TypeExpr::Generic(generic) => codegen.generic_type_name(generic),
        _ => "Int".to_string(),
    }
}

fn extract_string_literal(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Literal(Literal::String(value, _)) => Some(value.clone()),
        Expression::Literal(Literal::MultilineString(value, _)) => Some(value.clone()),
        Expression::Grouped(inner, _) => extract_string_literal(inner),
        _ => None,
    }
}

fn collect_struct_resolution_attributes(
    codegen: &CodeGen,
    struct_decl: &StructDecl,
    parent_structs: &[String],
) -> Result<StructResolutionAttributes> {
    let mut method_rules = HashMap::<String, String>::new();
    let mut conversion_rules = HashMap::<String, String>::new();

    for attr in &struct_decl.attributes {
        if attr.name != "resolve_inherit" {
            continue;
        }
        if attr.args.len() != 2 {
            return Err(anyhow!(
                "Attribute '@resolve_inherit' on struct '{}' expects 2 string arguments",
                struct_decl.name
            ));
        }

        let Some(selector) = extract_string_literal(&attr.args[0]) else {
            return Err(anyhow!(
                "Attribute '@resolve_inherit' on struct '{}' requires first argument to be a string literal",
                struct_decl.name
            ));
        };
        let Some(parent_name_raw) = extract_string_literal(&attr.args[1]) else {
            return Err(anyhow!(
                "Attribute '@resolve_inherit' on struct '{}' requires second argument to be a string literal",
                struct_decl.name
            ));
        };

        let selected_parent = codegen.resolve_struct_type_name(&parent_name_raw);
        if !parent_structs.iter().any(|p| p == &selected_parent) {
            return Err(anyhow!(
                "Attribute '@resolve_inherit' on struct '{}' references non-parent '{}'",
                struct_decl.name,
                parent_name_raw
            ));
        }

        if let Some(method_name) = selector.strip_prefix("method:") {
            if let Some(existing) =
                method_rules.insert(method_name.to_string(), selected_parent.clone())
            {
                if existing != selected_parent {
                    return Err(anyhow!(
                        "Conflicting @resolve_inherit entries for method '{}' on struct '{}'",
                        method_name,
                        struct_decl.name
                    ));
                }
            }
            continue;
        }
        if let Some(target_name_raw) = selector.strip_prefix("conv:") {
            let target_name = codegen.resolve_struct_type_name(target_name_raw);
            if let Some(existing) =
                conversion_rules.insert(target_name.clone(), selected_parent.clone())
            {
                if existing != selected_parent {
                    return Err(anyhow!(
                        "Conflicting @resolve_inherit entries for conversion '{}' on struct '{}'",
                        target_name,
                        struct_decl.name
                    ));
                }
            }
            continue;
        }

        return Err(anyhow!(
            "Invalid @resolve_inherit selector '{}' on struct '{}'. Use 'method:<name>' or 'conv:<Type>'.",
            selector,
            struct_decl.name
        ));
    }

    let method_rules = method_rules.into_iter().collect::<Vec<_>>();
    let conversion_rules = conversion_rules.into_iter().collect::<Vec<_>>();
    Ok((method_rules, conversion_rules))
}

mod compile;

pub use compile::compile_struct_methods;

pub fn mangle_operator(symbol: &str) -> String {
    match symbol {
        "+" => "add",
        "-" => "sub",
        "*" => "mul",
        "/" => "div",
        "%" => "mod",
        "==" => "eq",
        "!=" => "ne",
        "<" => "lt",
        "<=" => "le",
        ">" => "gt",
        ">=" => "ge",
        "|" => "bitor",
        "&" => "bitand",
        "^" => "bitxor",
        "shl" => "shl",
        "shr" => "shr",
        "&&" => "and",
        "||" => "or",
        _ => "op",
    }
    .to_string()
}

pub fn register_struct(codegen: &mut CodeGen, struct_decl: &StructDecl) -> Result<()> {
    let mut fields = Vec::new();
    let struct_fqcn = codegen.local_struct_fqcn(&struct_decl.name);
    let generic_param_names: Vec<String> = struct_decl
        .generic_params
        .iter()
        .map(|p| p.name.clone())
        .collect();
    let mut parent_structs: Vec<String> = Vec::new();
    let mut interfaces = Vec::new();
    let mut point_fields = Vec::new();

    for inherit in &struct_decl.inheritance {
        let inherit_name = codegen.type_expr_name(inherit);
        if codegen.interfaces.contains_key(&inherit_name) {
            interfaces.push(inherit_name);
            continue;
        }

        if codegen.type_registry.get(&inherit_name).is_none() {
            return Err(anyhow!(
                "Unknown inheritance target '{}' for struct '{}'",
                inherit_name,
                struct_decl.name
            ));
        }

        if inherit_name == struct_fqcn {
            return Err(anyhow!(
                "Struct '{}' cannot inherit itself",
                struct_decl.name
            ));
        }
        if parent_structs.iter().any(|name| name == &inherit_name) {
            return Err(anyhow!(
                "Duplicate concrete parent '{}' in struct '{}'",
                inherit_name,
                struct_decl.name
            ));
        }
        parent_structs.push(inherit_name);
    }

    let mut inherited_field_owners = HashMap::<String, String>::new();
    for parent in &parent_structs {
        if let Some(parent_info) = codegen.type_registry.get(parent) {
            for field in &parent_info.fields {
                if let Some(owner) = inherited_field_owners.get(&field.name) {
                    return Err(anyhow!(
                        "Ambiguous inherited field '{}' in struct '{}': '{}' and '{}'",
                        field.name,
                        struct_decl.name,
                        owner,
                        parent
                    ));
                }
                inherited_field_owners.insert(field.name.clone(), parent.clone());
                fields.push((field.name.clone(), field.type_name.clone(), field.is_public));
            }
        }
    }

    let (method_resolution, conversion_resolution) =
        collect_struct_resolution_attributes(codegen, struct_decl, &parent_structs)?;

    for member in &struct_decl.members {
        if let StructMember::Field(field) = member {
            let type_name = field_type_name(codegen, &field.type_expr, &generic_param_names);
            if fields.iter().any(|(name, _, _)| name == &field.name) {
                return Err(anyhow!(
                    "Duplicate field '{}' in struct '{}'",
                    field.name,
                    struct_decl.name
                ));
            }
            let is_public = matches!(field.visibility, Visibility::Public);
            fields.push((field.name.clone(), type_name, is_public));
            if field.is_point {
                point_fields.push((
                    field.name.clone(),
                    field_type_name(codegen, &field.type_expr, &generic_param_names),
                ));
            }
        }
    }

    codegen.type_registry.register_struct(&struct_fqcn, fields);
    let is_public = matches!(struct_decl.visibility, Visibility::Public);
    codegen
        .struct_visibility
        .insert(struct_fqcn.clone(), is_public);
    codegen
        .struct_packages
        .insert(struct_fqcn, codegen.current_package.clone());
    if !struct_decl.generic_params.is_empty() {
        codegen.generic_struct_params.insert(
            codegen.local_struct_fqcn(&struct_decl.name),
            struct_decl.generic_params.clone(),
        );
    }
    if !parent_structs.is_empty() {
        codegen
            .struct_parents
            .insert(codegen.local_struct_fqcn(&struct_decl.name), parent_structs);
    }
    let struct_name = codegen.local_struct_fqcn(&struct_decl.name);
    for (method_name, selected_parent) in method_resolution {
        codegen
            .struct_method_resolution
            .insert((struct_name.clone(), method_name), selected_parent);
    }
    for (target_name, selected_parent) in conversion_resolution {
        codegen
            .struct_conversion_resolution
            .insert((struct_name.clone(), target_name), selected_parent);
    }
    if !interfaces.is_empty() {
        codegen
            .struct_interfaces
            .insert(codegen.local_struct_fqcn(&struct_decl.name), interfaces);
    }
    if !point_fields.is_empty() {
        codegen
            .struct_point_fields
            .insert(codegen.local_struct_fqcn(&struct_decl.name), point_fields);
    }
    Ok(())
}

fn struct_symbol_owner(codegen: &CodeGen, struct_decl: &StructDecl) -> String {
    codegen.local_struct_fqcn(&struct_decl.name)
}

fn is_ancestor_struct(codegen: &CodeGen, owner: &str, candidate_parent: &str) -> bool {
    let mut stack = codegen
        .struct_parents
        .get(owner)
        .cloned()
        .unwrap_or_default();
    let mut visited = std::collections::HashSet::new();
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if current == candidate_parent {
            return true;
        }
        if let Some(parents) = codegen.struct_parents.get(&current) {
            for parent in parents {
                stack.push(parent.clone());
            }
        }
    }
    false
}

fn collect_inherited_method_owners(
    codegen: &CodeGen,
    owner: &str,
    method_name: &str,
) -> Vec<String> {
    let mut matches = Vec::new();
    let mut stack = codegen
        .struct_parents
        .get(owner)
        .cloned()
        .unwrap_or_default();
    let mut visited = std::collections::HashSet::new();

    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        let symbol = format!("{}__{}", current, method_name);
        if codegen.functions.contains_key(&symbol) {
            matches.push(current.clone());
        }
        if let Some(parents) = codegen.struct_parents.get(&current) {
            for parent in parents {
                stack.push(parent.clone());
            }
        }
    }

    matches
}

fn validate_method_inherit_attribute(
    codegen: &CodeGen,
    struct_decl: &StructDecl,
    owner: &str,
    method: &MethodDecl,
) -> Result<()> {
    let inherit_attrs = method
        .attributes
        .iter()
        .filter(|attr| attr.name == "inherit_from")
        .collect::<Vec<_>>();
    if inherit_attrs.is_empty() {
        return Ok(());
    }
    if inherit_attrs.len() > 1 {
        return Err(anyhow!(
            "Method '{}.{}' has duplicate '@inherit_from' attributes",
            struct_decl.name,
            method.name
        ));
    }

    let attr = inherit_attrs[0];
    if attr.args.len() != 1 {
        return Err(anyhow!(
            "Attribute '@inherit_from' on method '{}.{}' expects 1 string argument",
            struct_decl.name,
            method.name
        ));
    }

    let Some(parent_raw) = extract_string_literal(&attr.args[0]) else {
        return Err(anyhow!(
            "Attribute '@inherit_from' on method '{}.{}' requires a string literal argument",
            struct_decl.name,
            method.name
        ));
    };

    let selected_parent = codegen.resolve_struct_type_name(&parent_raw);
    if !is_ancestor_struct(codegen, owner, &selected_parent) {
        return Err(anyhow!(
            "Attribute '@inherit_from' on method '{}.{}' references non-parent '{}'",
            struct_decl.name,
            method.name,
            parent_raw
        ));
    }

    let inherited = collect_inherited_method_owners(codegen, owner, &method.name);
    if inherited.is_empty() {
        return Err(anyhow!(
            "Attribute '@inherit_from' on method '{}.{}' requires overriding an inherited method",
            struct_decl.name,
            method.name
        ));
    }

    if !inherited.iter().any(|name| name == &selected_parent) {
        return Err(anyhow!(
            "Attribute '@inherit_from' on method '{}.{}' references '{}' but inherited methods are from: {}",
            struct_decl.name,
            method.name,
            selected_parent,
            inherited.join(", ")
        ));
    }

    Ok(())
}

pub fn declare_struct_methods(codegen: &mut CodeGen, struct_decl: &StructDecl) -> Result<()> {
    let owner = struct_symbol_owner(codegen, struct_decl);
    for member in &struct_decl.members {
        match member {
            StructMember::Method(method) => {
                validate_method_inherit_attribute(codegen, struct_decl, &owner, method)?;
                let func_name = format!("{}__{}", owner, method.name);

                let mut sig = codegen.module.make_signature();

                if let Some(ref user_ret) = method.return_type {
                    if method.throws {
                        let actual_ret = wrap_return_type(user_ret);
                        codegen
                            .function_return_types
                            .insert(func_name.clone(), actual_ret.clone());
                        sig.returns.push(AbiParam::new(types::I64));
                        if let TypeExpr::Tuple(tt) = &actual_ret {
                            if tt.element_types.len() >= 2 {
                                if let Some(cl) =
                                    codegen.convert_type(&tt.element_types[1].type_expr)
                                {
                                    sig.returns.push(AbiParam::new(cl));
                                } else {
                                    sig.returns.push(AbiParam::new(types::I64));
                                }
                            }
                        }
                    } else if let Some(cl_type) = codegen.convert_type(user_ret) {
                        sig.returns.push(AbiParam::new(cl_type));
                    }
                }

                sig.params.push(AbiParam::new(types::I64));
                let mut param_types = vec![VarType::Struct(
                    codegen.local_struct_fqcn(&struct_decl.name),
                )];
                let mut param_type_exprs = Vec::new();

                for param in &method.params {
                    if let MethodParam::Regular(p) = param {
                        if let Some(cl_type) = codegen.convert_type(&p.type_expr) {
                            sig.params.push(AbiParam::new(cl_type));
                        }
                        param_types.push(
                            crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                                codegen,
                                &p.type_expr,
                            ),
                        );
                        param_type_exprs.push(p.type_expr.clone());
                    }
                }

                let func_id = codegen
                    .module
                    .declare_function(&func_name, Linkage::Local, &sig)
                    .map_err(|e| anyhow!("Failed to declare method {}: {}", func_name, e))?;

                codegen.functions.insert(func_name.clone(), func_id);
                let method_ret_type = method
                    .return_type
                    .as_ref()
                    .map(|t| {
                        crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(codegen, t)
                    })
                    .unwrap_or(VarType::Unknown);
                codegen
                    .function_value_types
                    .insert(func_name.clone(), method_ret_type);
                codegen
                    .function_param_types
                    .insert(func_name.clone(), param_types);
                codegen
                    .function_param_type_exprs
                    .insert(func_name.clone(), param_type_exprs);
                if !method.generic_params.is_empty() {
                    codegen
                        .function_generic_params
                        .insert(func_name.clone(), method.generic_params.clone());
                }
                let is_pub = matches!(method.visibility, Visibility::Public);
                codegen.function_visibility.insert(func_name, is_pub);
            }
            StructMember::Constructor(ctor) => {
                let func_name = format!("{}__new", owner);
                let mut sig = codegen.module.make_signature();
                let mut param_types = Vec::new();

                if ctor.throws {
                    let ptr_ty = TypeExpr::Base(BaseType {
                        name: "Int".to_string(),
                        span: Span { start: 0, end: 0 },
                    });
                    let actual_ret = wrap_return_type(&ptr_ty);
                    codegen
                        .function_return_types
                        .insert(func_name.clone(), actual_ret);
                    sig.returns.push(AbiParam::new(types::I64));
                    sig.returns.push(AbiParam::new(types::I64));
                } else {
                    sig.returns.push(AbiParam::new(types::I64));
                }
                for p in &ctor.params {
                    if let Some(cl_type) = codegen.convert_type(&p.type_expr) {
                        sig.params.push(AbiParam::new(cl_type));
                    }
                    param_types.push(
                        crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                            codegen,
                            &p.type_expr,
                        ),
                    );
                }
                let func_id = codegen
                    .module
                    .declare_function(&func_name, Linkage::Local, &sig)
                    .map_err(|e| anyhow!("Failed to declare constructor {}: {}", func_name, e))?;
                codegen.functions.insert(func_name.clone(), func_id);
                codegen.function_value_types.insert(
                    func_name.clone(),
                    VarType::Struct(codegen.local_struct_fqcn(&struct_decl.name)),
                );
                codegen
                    .function_param_types
                    .insert(func_name.clone(), param_types);
                let is_pub = matches!(ctor.visibility, Visibility::Public);
                codegen.function_visibility.insert(func_name, is_pub);
            }
            StructMember::Default(default_decl) => {
                let func_name = format!("{}__default", owner);
                let mut sig = codegen.module.make_signature();
                sig.returns.push(AbiParam::new(types::I64));

                let func_id = codegen
                    .module
                    .declare_function(&func_name, Linkage::Local, &sig)
                    .map_err(|e| anyhow!("Failed to declare default {}: {}", func_name, e))?;

                codegen.functions.insert(func_name.clone(), func_id);
                codegen.function_value_types.insert(
                    func_name.clone(),
                    VarType::Struct(codegen.local_struct_fqcn(&struct_decl.name)),
                );
                codegen
                    .function_param_types
                    .insert(func_name.clone(), vec![]);
                let is_pub = matches!(default_decl.visibility, Visibility::Public);
                codegen.function_visibility.insert(func_name, is_pub);
            }
            StructMember::Operator(op) => {
                let mangled = mangle_operator(&op.op_symbol);
                let func_name = format!("{}__op_{}", owner, mangled);
                let struct_fqcn = owner.clone();
                codegen
                    .struct_operators
                    .insert((struct_fqcn, op.op_symbol.clone()), func_name.clone());
                let mut sig = codegen.module.make_signature();
                if let Some(ref user_ret) = op.return_type {
                    if op.throws {
                        let actual_ret = wrap_return_type(user_ret);
                        codegen
                            .function_return_types
                            .insert(func_name.clone(), actual_ret.clone());
                        sig.returns.push(AbiParam::new(types::I64));
                        if let TypeExpr::Tuple(tt) = &actual_ret {
                            if tt.element_types.len() >= 2 {
                                if let Some(cl) =
                                    codegen.convert_type(&tt.element_types[1].type_expr)
                                {
                                    sig.returns.push(AbiParam::new(cl));
                                } else {
                                    sig.returns.push(AbiParam::new(types::I64));
                                }
                            }
                        }
                    } else if let Some(cl_type) = codegen.convert_type(user_ret) {
                        sig.returns.push(AbiParam::new(cl_type));
                    }
                }
                sig.params.push(AbiParam::new(types::I64));
                let mut param_types = vec![VarType::Struct(
                    codegen.local_struct_fqcn(&struct_decl.name),
                )];
                for param in &op.params {
                    if let Some(cl_type) = codegen.convert_type(&param.type_expr) {
                        sig.params.push(AbiParam::new(cl_type));
                    }
                    param_types.push(
                        crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                            codegen,
                            &param.type_expr,
                        ),
                    );
                }
                let func_id = codegen
                    .module
                    .declare_function(&func_name, Linkage::Local, &sig)
                    .map_err(|e| anyhow!("Failed to declare operator {}: {}", func_name, e))?;
                codegen.functions.insert(func_name.clone(), func_id);
                let op_ret_type = op
                    .return_type
                    .as_ref()
                    .map(|t| {
                        crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(codegen, t)
                    })
                    .unwrap_or(VarType::Unknown);
                codegen
                    .function_value_types
                    .insert(func_name.clone(), op_ret_type);
                codegen
                    .function_param_types
                    .insert(func_name.clone(), param_types);
                let is_pub = matches!(op.visibility, Visibility::Public);
                codegen.function_visibility.insert(func_name, is_pub);
            }
            StructMember::Conversion(conv) => {
                let target_name = match &conv.target_type {
                    TypeExpr::Base(b) => codegen.resolve_struct_type_name(&b.name),
                    _ => continue,
                };
                let func_name = format!("{}__to_{}", owner, target_name);
                let struct_fqcn = owner.clone();
                codegen
                    .struct_conversions
                    .insert((struct_fqcn, target_name), func_name.clone());
                let mut sig = codegen.module.make_signature();
                if let Some(cl_type) = codegen.convert_type(&conv.target_type) {
                    sig.returns.push(AbiParam::new(cl_type));
                }
                sig.params.push(AbiParam::new(types::I64));
                let param_types = vec![VarType::Struct(
                    codegen.local_struct_fqcn(&struct_decl.name),
                )];
                let func_id = codegen
                    .module
                    .declare_function(&func_name, Linkage::Local, &sig)
                    .map_err(|e| anyhow!("Failed to declare conversion {}: {}", func_name, e))?;
                codegen.functions.insert(func_name.clone(), func_id);
                codegen.function_value_types.insert(
                    func_name.clone(),
                    crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                        codegen,
                        &conv.target_type,
                    ),
                );
                codegen
                    .function_param_types
                    .insert(func_name.clone(), param_types);
                let is_pub = matches!(conv.visibility, Visibility::Public);
                codegen.function_visibility.insert(func_name, is_pub);
            }
            StructMember::Field(_) => {}
        }
    }
    validate_and_materialize_interface_conversions(codegen, struct_decl)?;
    Ok(())
}

fn validate_and_materialize_interface_conversions(
    codegen: &mut CodeGen,
    struct_decl: &StructDecl,
) -> Result<()> {
    let struct_name = codegen.local_struct_fqcn(&struct_decl.name);
    let Some(interfaces) = codegen.struct_interfaces.get(&struct_name) else {
        return Ok(());
    };

    let interfaces = interfaces.clone();
    let mut default_candidates: HashMap<String, Vec<(String, InterfaceDefaultConversion)>> =
        HashMap::new();

    for interface_name in &interfaces {
        let Some(interface_info) = codegen.interfaces.get(interface_name) else {
            continue;
        };

        for target_name in &interface_info.conversions {
            if let Some(default_conv) = interface_info.default_conversions.get(target_name) {
                default_candidates
                    .entry(target_name.clone())
                    .or_default()
                    .push((interface_name.clone(), default_conv.clone()));
            }
        }
    }

    let required_targets: Vec<String> = default_candidates
        .keys()
        .cloned()
        .chain(
            interfaces
                .iter()
                .filter_map(|iface| codegen.interfaces.get(iface))
                .flat_map(|info| info.conversions.iter().cloned()),
        )
        .collect();

    for target_name in required_targets {
        if codegen
            .try_resolve_struct_conversion_name(&struct_name, &target_name)?
            .is_some()
        {
            continue;
        }

        let defaults = default_candidates
            .get(&target_name)
            .cloned()
            .unwrap_or_default();
        if defaults.is_empty() {
            return Err(anyhow!(
                "Struct '{}' does not implement required interface conversion 'to {}'",
                struct_decl.name,
                target_name
            ));
        }

        if defaults.len() > 1 {
            let providers = defaults
                .iter()
                .map(|(iface, _)| iface.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow!(
                "Ambiguous default interface conversion for '{}' -> '{}': {}. Implement conversion explicitly in struct '{}'.",
                struct_decl.name,
                target_name,
                providers,
                struct_decl.name
            ));
        }

        let (_iface, default_conv) = defaults.into_iter().next().unwrap();
        let func_name = format!("{}__to_{}", struct_name, target_name);
        if !codegen.functions.contains_key(&func_name) {
            let mut sig = codegen.module.make_signature();
            if let Some(cl_type) = codegen.convert_type(&default_conv.target_type) {
                sig.returns.push(AbiParam::new(cl_type));
            }
            sig.params.push(AbiParam::new(types::I64));
            let func_id = codegen
                .module
                .declare_function(&func_name, Linkage::Local, &sig)
                .map_err(|e| {
                    anyhow!(
                        "Failed to declare synthesized conversion {}: {}",
                        func_name,
                        e
                    )
                })?;
            codegen.functions.insert(func_name.clone(), func_id);
            codegen.function_value_types.insert(
                func_name.clone(),
                crate::codegen::expression::typing::infer_type_expr_to_var_type_with_codegen(
                    codegen,
                    &default_conv.target_type,
                ),
            );
            codegen.function_param_types.insert(
                func_name.clone(),
                vec![VarType::Struct(struct_name.clone())],
            );
            codegen.function_visibility.insert(func_name.clone(), true);
        }

        codegen.struct_conversions.insert(
            (struct_name.clone(), target_name.clone()),
            func_name.clone(),
        );
        codegen.synthesized_conversions.insert(
            func_name,
            crate::codegen::SynthesizedConversion {
                owner: struct_name.clone(),
                target_type: default_conv.target_type,
                body: default_conv.body,
            },
        );
    }

    let unique_required = interfaces
        .iter()
        .filter_map(|iface| codegen.interfaces.get(iface))
        .flat_map(|info| info.conversions.iter().cloned())
        .collect::<std::collections::HashSet<_>>();

    for target_name in unique_required {
        if codegen
            .try_resolve_struct_conversion_name(&struct_name, &target_name)?
            .is_none()
        {
            return Err(anyhow!(
                "Struct '{}' does not implement required interface conversion 'to {}'",
                struct_decl.name,
                target_name
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::mangle_operator;

    #[test]
    fn mangle_operator_maps_known_symbols() {
        assert_eq!(mangle_operator("+"), "add");
        assert_eq!(mangle_operator("-"), "sub");
        assert_eq!(mangle_operator("*"), "mul");
        assert_eq!(mangle_operator("/"), "div");
        assert_eq!(mangle_operator("=="), "eq");
        assert_eq!(mangle_operator("shl"), "shl");
        assert_eq!(mangle_operator("&&"), "and");
    }

    #[test]
    fn mangle_operator_falls_back_for_unknown() {
        assert_eq!(mangle_operator("???"), "op");
    }
}
