use super::*;

pub(crate) fn split_generic_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;

    for (i, ch) in input.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(input[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

pub(crate) fn substitute_type_name(type_name: &str, subst: &HashMap<String, String>) -> String {
    if let Some(mapped) = subst.get(type_name) {
        return mapped.clone();
    }

    if let Some(inner) = type_name.strip_prefix("[]") {
        return format!("[]{}", substitute_type_name(inner, subst));
    }

    if let Some(inner) = type_name
        .strip_prefix("Channel<")
        .and_then(|s| s.strip_suffix('>'))
    {
        return format!("Channel<{}>", substitute_type_name(inner, subst));
    }

    if let Some(rest) = type_name.strip_prefix('[') {
        if let Some((len_part, inner)) = rest.split_once(']') {
            return format!("[{}]{}", len_part, substitute_type_name(inner, subst));
        }
    }

    type_name.to_string()
}

#[cfg(test)]
pub fn is_error_result_type(ty: &TypeExpr) -> bool {
    if let TypeExpr::Tuple(tt) = ty {
        if tt.element_types.len() >= 2 {
            if let TypeExpr::Base(b) = tt.element_types[0].type_expr.as_ref() {
                return b.name == "Error";
            }
        }
    }
    false
}

pub fn wrap_return_type(user_ty: &TypeExpr) -> TypeExpr {
    let span = Span { start: 0, end: 0 };
    let err_ty = TypeExpr::Base(BaseType {
        name: "Error".to_string(),
        span: span.clone(),
    });
    let err_tc = TypeConstraint {
        type_expr: Box::new(err_ty),
        constraints: vec![],
        span: span.clone(),
    };
    let val_tc = TypeConstraint {
        type_expr: Box::new(user_ty.clone()),
        constraints: vec![],
        span: span.clone(),
    };
    TypeExpr::Tuple(TupleType {
        element_types: vec![err_tc, val_tc],
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(name: &str) -> TypeExpr {
        TypeExpr::Base(BaseType {
            name: name.to_string(),
            span: Span { start: 0, end: 0 },
        })
    }

    fn type_constraint(ty: TypeExpr) -> TypeConstraint {
        TypeConstraint {
            type_expr: Box::new(ty),
            constraints: vec![],
            span: Span { start: 0, end: 0 },
        }
    }

    #[test]
    fn wrap_return_type_creates_error_tuple() {
        let wrapped = wrap_return_type(&base("Int"));
        assert!(is_error_result_type(&wrapped));

        match wrapped {
            TypeExpr::Tuple(tuple) => {
                assert_eq!(tuple.element_types.len(), 2);
                match tuple.element_types[0].type_expr.as_ref() {
                    TypeExpr::Base(b) => assert_eq!(b.name, "Error"),
                    _ => panic!("first type must be Error"),
                }
                match tuple.element_types[1].type_expr.as_ref() {
                    TypeExpr::Base(b) => assert_eq!(b.name, "Int"),
                    _ => panic!("second type must be original type"),
                }
            }
            _ => panic!("wrap_return_type must return tuple"),
        }
    }

    #[test]
    fn is_error_result_type_detects_only_error_tuple_shape() {
        let valid = TypeExpr::Tuple(TupleType {
            element_types: vec![type_constraint(base("Error")), type_constraint(base("Int"))],
            span: Span { start: 0, end: 0 },
        });
        let invalid = TypeExpr::Tuple(TupleType {
            element_types: vec![type_constraint(base("Int")), type_constraint(base("Error"))],
            span: Span { start: 0, end: 0 },
        });

        assert!(is_error_result_type(&valid));
        assert!(!is_error_result_type(&invalid));
        assert!(!is_error_result_type(&base("Int")));
    }

    #[test]
    fn mangling_and_access_checks_follow_package_visibility_rules() {
        let mut codegen = CodeGen::new().unwrap();
        codegen.set_entry_package("main");
        codegen.current_package = "main".to_string();
        assert_eq!(codegen.mangle_name("main"), "main");

        codegen.current_package = "pkg".to_string();
        assert_eq!(codegen.mangle_name("foo"), "pkg.foo");

        codegen
            .function_visibility
            .insert("other.secret".to_string(), false);
        let err = codegen.check_function_access("other.secret").unwrap_err();
        assert!(err.to_string().contains("Cannot access private function"));

        codegen.current_package = "other".to_string();
        assert!(codegen.check_function_access("other.secret").is_ok());
    }

    #[test]
    fn field_access_check_respects_public_flag() {
        let mut codegen = CodeGen::new().unwrap();
        codegen.current_package = "consumer".to_string();
        codegen
            .struct_packages
            .insert("Point".to_string(), "model".to_string());
        codegen.type_registry.register_struct(
            "Point",
            vec![
                ("x".to_string(), "Int".to_string(), true),
                ("y".to_string(), "Int".to_string(), false),
            ],
        );

        assert!(codegen.check_field_access("Point", "x").is_ok());
        let err = codegen.check_field_access("Point", "y").unwrap_err();
        assert!(err.to_string().contains("Cannot access private field"));
    }
}
