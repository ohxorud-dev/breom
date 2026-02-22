use crate::Rule;
use pest::iterators::Pair;

use super::common::Span;
use super::expressions::Expression;
use super::types::TypeExpr;

#[derive(Debug, Clone)]
pub struct AttributeDecl {
    pub name: String,
    pub params: Vec<AttributeParam>,
    pub span: Span,
}

impl AttributeDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::attribute_decl);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut params = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::attribute_decl_params => {
                    if let Some(list) = inner.into_inner().next() {
                        for param in list.into_inner() {
                            params.push(AttributeParam::from_pair(param));
                        }
                    }
                }
                _ => {}
            }
        }

        AttributeDecl { name, params, span }
    }
}

#[derive(Debug, Clone)]
pub struct AttributeParam {
    pub name: String,
    pub param_type: TypeExpr,
    pub span: Span,
}

impl AttributeParam {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::attribute_param);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut param_type = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::type_expr => param_type = Some(TypeExpr::from_pair(inner)),
                _ => {}
            }
        }

        AttributeParam {
            name,
            param_type: param_type.expect("attribute param must have type"),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

impl Attribute {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::attribute);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut args = Vec::new();

        if let Some(inner_attr) = pair.into_inner().next() {
            match inner_attr.as_rule() {
                Rule::attribute_no_args => {
                    if let Some(inner) = inner_attr.into_inner().next() {
                        if inner.as_rule() == Rule::identifier {
                            name = inner.as_str().to_string();
                        }
                    }
                }
                Rule::attribute_with_args => {
                    for inner in inner_attr.into_inner() {
                        match inner.as_rule() {
                            Rule::identifier => name = inner.as_str().to_string(),
                            Rule::arg_list => {
                                for arg in inner.into_inner() {
                                    args.push(Expression::from_pair(arg));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Attribute { name, args, span }
    }
}
