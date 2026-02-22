use tower_lsp::lsp_types::{DocumentSymbol, SymbolKind};

use super::analysis::ast_span_to_range;
use crate::ast::{
    declarations::{
        DefaultMethod, EnumVariantDecl, FunctionDecl, InterfaceMember, MethodDecl, MethodParam,
        MethodSignature, StructMember,
    },
    program::{Program, TopLevelItem},
    types::{TypeConstraint, TypeExpr},
};

pub fn collect_document_symbols(content: &str, program: &Program) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    for item in &program.items {
        match item {
            TopLevelItem::Function(f) => {
                symbols.push(make_symbol(
                    &f.name,
                    SymbolKind::FUNCTION,
                    Some(format_function_signature(f)),
                    ast_span_to_range(content, &f.span),
                    None,
                ));
            }
            TopLevelItem::Struct(s) => {
                let mut children = Vec::new();
                for member in &s.members {
                    match member {
                        StructMember::Field(field) => {
                            children.push(make_symbol(
                                &field.name,
                                SymbolKind::FIELD,
                                Some(format_type_expr(&field.type_expr)),
                                ast_span_to_range(content, &field.span),
                                None,
                            ));
                        }
                        StructMember::Method(method) => {
                            children.push(make_symbol(
                                &method.name,
                                SymbolKind::METHOD,
                                Some(format_method_signature(method)),
                                ast_span_to_range(content, &method.span),
                                None,
                            ));
                        }
                        StructMember::Constructor(ctor) => {
                            let detail = format!(
                                "ctor({})",
                                ctor.params
                                    .iter()
                                    .map(|p| format!(
                                        "{} {}",
                                        p.name,
                                        format_type_expr(&p.type_expr)
                                    ))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                            children.push(make_symbol(
                                "constructor",
                                SymbolKind::CONSTRUCTOR,
                                Some(detail),
                                ast_span_to_range(content, &ctor.span),
                                None,
                            ));
                        }
                        _ => {}
                    }
                }

                let children = if children.is_empty() {
                    None
                } else {
                    Some(children)
                };
                symbols.push(make_symbol(
                    &s.name,
                    SymbolKind::STRUCT,
                    None,
                    ast_span_to_range(content, &s.span),
                    children,
                ));
            }
            TopLevelItem::Interface(i) => {
                let mut children = Vec::new();
                for member in &i.members {
                    match member {
                        InterfaceMember::Signature(sig) => children.push(make_symbol(
                            &sig.name,
                            SymbolKind::METHOD,
                            Some(format_interface_signature(sig)),
                            ast_span_to_range(content, &sig.span),
                            None,
                        )),
                        InterfaceMember::DefaultMethod(default) => children.push(make_symbol(
                            &default.name,
                            SymbolKind::METHOD,
                            Some(format_default_method_signature(default)),
                            ast_span_to_range(content, &default.span),
                            None,
                        )),
                        InterfaceMember::ConversionSignature(conv) => children.push(make_symbol(
                            "conversion",
                            SymbolKind::METHOD,
                            Some(format!("to {}", format_type_expr(&conv.target_type))),
                            ast_span_to_range(content, &conv.span),
                            None,
                        )),
                        InterfaceMember::DefaultConversion(conv) => children.push(make_symbol(
                            "default conversion",
                            SymbolKind::METHOD,
                            Some(format!("to {}", format_type_expr(&conv.target_type))),
                            ast_span_to_range(content, &conv.span),
                            None,
                        )),
                    }
                }

                let children = if children.is_empty() {
                    None
                } else {
                    Some(children)
                };
                symbols.push(make_symbol(
                    &i.name,
                    SymbolKind::INTERFACE,
                    None,
                    ast_span_to_range(content, &i.span),
                    children,
                ));
            }
            TopLevelItem::Enum(e) => {
                let mut children = Vec::new();
                for variant in &e.variants {
                    children.push(make_symbol(
                        &variant.name,
                        SymbolKind::ENUM_MEMBER,
                        enum_variant_detail(variant),
                        ast_span_to_range(content, &variant.span),
                        None,
                    ));
                }
                let children = if children.is_empty() {
                    None
                } else {
                    Some(children)
                };

                symbols.push(make_symbol(
                    &e.name,
                    SymbolKind::ENUM,
                    None,
                    ast_span_to_range(content, &e.span),
                    children,
                ));
            }
            TopLevelItem::Define(d) => {
                let detail = d.type_annotation.as_ref().map(format_type_expr);
                symbols.push(make_symbol(
                    &d.name,
                    SymbolKind::CONSTANT,
                    detail,
                    ast_span_to_range(content, &d.span),
                    None,
                ));
            }
            _ => {}
        }
    }

    symbols
}

#[allow(deprecated)]
fn make_symbol(
    name: &str,
    kind: SymbolKind,
    detail: Option<String>,
    range: tower_lsp::lsp_types::Range,
    children: Option<Vec<DocumentSymbol>>,
) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children,
    }
}

fn format_function_signature(function: &FunctionDecl) -> String {
    let params = function
        .params
        .iter()
        .map(|p| format!("{} {}", p.name, format_type_expr(&p.type_expr)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!("fn({params})");
    if let Some(ret) = &function.return_type {
        out.push_str(" -> ");
        out.push_str(&format_type_expr(ret));
    }
    if function.throws {
        out.push_str(" throws");
    }
    out
}

fn format_method_signature(method: &MethodDecl) -> String {
    let params = method
        .params
        .iter()
        .map(format_method_param)
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!("fn({params})");
    if let Some(ret) = &method.return_type {
        out.push_str(" -> ");
        out.push_str(&format_type_expr(ret));
    }
    if method.throws {
        out.push_str(" throws");
    }
    out
}

fn format_interface_signature(sig: &MethodSignature) -> String {
    let params = sig
        .params
        .iter()
        .map(format_method_param)
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!("fn({params})");
    if let Some(ret) = &sig.return_type {
        out.push_str(" -> ");
        out.push_str(&format_type_expr(ret));
    }
    if sig.throws {
        out.push_str(" throws");
    }
    out
}

fn format_default_method_signature(default: &DefaultMethod) -> String {
    let params = default
        .params
        .iter()
        .map(format_method_param)
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!("fn({params})");
    if let Some(ret) = &default.return_type {
        out.push_str(" -> ");
        out.push_str(&format_type_expr(ret));
    }
    if default.throws {
        out.push_str(" throws");
    }
    out
}

fn enum_variant_detail(variant: &EnumVariantDecl) -> Option<String> {
    if variant.payload_types.is_empty() {
        return None;
    }
    Some(
        variant
            .payload_types
            .iter()
            .map(format_type_expr)
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn format_method_param(param: &MethodParam) -> String {
    match param {
        MethodParam::SelfParam => "self".to_string(),
        MethodParam::Regular(p) => format!("{} {}", p.name, format_type_expr(&p.type_expr)),
    }
}

fn format_type_constraint(tc: &TypeConstraint) -> String {
    let mut out = format_type_expr(&tc.type_expr);
    if !tc.constraints.is_empty() {
        out.push(':');
        out.push_str(
            &tc.constraints
                .iter()
                .map(format_type_expr)
                .collect::<Vec<_>>()
                .join(" | "),
        );
    }
    out
}

fn format_type_expr(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Base(base) => base.name.clone(),
        TypeExpr::Generic(generic) => {
            let args = generic
                .type_args
                .iter()
                .map(format_type_constraint)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", generic.base, args)
        }
        TypeExpr::Array(arr) => format!("[{}]{}", arr.size, format_type_expr(&arr.element_type)),
        TypeExpr::DynamicArray(arr) => format!("[]{}", format_type_expr(&arr.element_type)),
        TypeExpr::Chan(chan) => format!("chan {}", format_type_expr(&chan.element_type)),
        TypeExpr::Tuple(tuple) => format!(
            "({})",
            tuple
                .element_types
                .iter()
                .map(format_type_constraint)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypeExpr::Function(func) => {
            let params = func
                .param_types
                .iter()
                .map(format_type_expr)
                .collect::<Vec<_>>()
                .join(", ");
            if let Some(ret) = &func.return_type {
                format!("fn({params}) -> {}", format_type_expr(ret))
            } else {
                format!("fn({params})")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::analysis::parse_and_collect_diagnostics;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn collect_document_symbols_includes_nested_types() {
        let content = r#"
            define LIMIT Int = 10

            struct Point {
                pub x Int
                y Int

                pub fn move(dx Int, dy Int) {
                }
            }

            enum Result {
                Ok(Int)
                Err(String)
            }

            fn main() {
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diagnostics) = parse_and_collect_diagnostics(content, &uri);
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );

        let symbols = collect_document_symbols(content, &program.unwrap());
        assert!(symbols.iter().any(|s| s.name == "LIMIT"));
        assert!(symbols.iter().any(|s| s.name == "main"));

        let point = symbols.iter().find(|s| s.name == "Point").unwrap();
        let point_children = point.children.as_ref().unwrap();
        assert!(point_children.iter().any(|s| s.name == "x"));
        assert!(point_children.iter().any(|s| s.name == "move"));

        let result_enum = symbols.iter().find(|s| s.name == "Result").unwrap();
        let enum_children = result_enum.children.as_ref().unwrap();
        assert!(enum_children.iter().any(|s| s.name == "Ok"));
        assert!(enum_children.iter().any(|s| s.name == "Err"));
    }
}
