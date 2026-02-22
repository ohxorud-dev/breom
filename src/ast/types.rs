use crate::Rule;
use pest::iterators::Pair;

use super::common::Span;
use super::expressions::parse_integer;

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Base(BaseType),
    Generic(GenericType),
    Array(ArrayType),
    DynamicArray(DynamicArrayType),
    Chan(ChanType),
    Tuple(TupleType),
    Function(FunctionType),
}

impl TypeExpr {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let inner = match pair.as_rule() {
            Rule::type_expr => pair.into_inner().next().unwrap(),
            Rule::type_constraint => {
                return TypeExpr::from_pair(pair.into_inner().next().unwrap());
            }
            _ => pair,
        };

        match inner.as_rule() {
            Rule::base_type => TypeExpr::Base(BaseType::from_pair(inner)),
            Rule::generic_type => TypeExpr::Generic(GenericType::from_pair(inner)),
            Rule::array_type => TypeExpr::Array(ArrayType::from_pair(inner)),
            Rule::dynamic_array_type => TypeExpr::DynamicArray(DynamicArrayType::from_pair(inner)),
            Rule::chan_type => TypeExpr::Chan(ChanType::from_pair(inner)),
            Rule::tuple_type => TypeExpr::Tuple(TupleType::from_pair(inner)),
            Rule::function_type => TypeExpr::Function(FunctionType::from_pair(inner)),
            _ => unreachable!("Unexpected type rule: {:?}", inner.as_rule()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BaseType {
    pub name: String,
    pub span: Span,
}

impl BaseType {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::base_type);
        let span = Span::from_pest(pair.as_span());
        BaseType {
            name: pair.as_str().to_string(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenericType {
    pub base: String,
    pub type_args: Vec<TypeConstraint>,
    pub span: Span,
}

impl GenericType {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::generic_type);
        let span = Span::from_pest(pair.as_span());

        let mut base = String::new();
        let mut type_args = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::base_type => base = inner.as_str().to_string(),
                Rule::type_list => {
                    for tc in inner.into_inner() {
                        type_args.push(TypeConstraint::from_pair(tc));
                    }
                }
                _ => {}
            }
        }

        GenericType {
            base,
            type_args,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub type_expr: Box<TypeExpr>,
    pub constraints: Vec<TypeExpr>,
    pub span: Span,
}

impl TypeConstraint {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::type_constraint);
        let span = Span::from_pest(pair.as_span());

        let mut type_expr = None;
        let mut constraints = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::type_expr => type_expr = Some(Box::new(TypeExpr::from_pair(inner))),
                Rule::constraint_list => {
                    for c in inner.into_inner() {
                        constraints.push(TypeExpr::from_pair(c));
                    }
                }
                _ => {}
            }
        }

        TypeConstraint {
            type_expr: type_expr.unwrap(),
            constraints,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub element_type: Box<TypeExpr>,
    pub size: u64,
    pub span: Span,
}

impl ArrayType {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::array_type);
        let span = Span::from_pest(pair.as_span());

        let mut element_type = None;
        let mut size = 0;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::integer_literal => size = parse_integer(inner.as_str()),
                Rule::type_expr => element_type = Some(Box::new(TypeExpr::from_pair(inner))),
                _ => {}
            }
        }

        ArrayType {
            element_type: element_type.unwrap(),
            size,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DynamicArrayType {
    pub element_type: Box<TypeExpr>,
    pub span: Span,
}

impl DynamicArrayType {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::dynamic_array_type);
        let span = Span::from_pest(pair.as_span());
        let element_type = Box::new(TypeExpr::from_pair(pair.into_inner().next().unwrap()));
        DynamicArrayType { element_type, span }
    }
}

#[derive(Debug, Clone)]
pub struct ChanType {
    pub element_type: Box<TypeExpr>,
    pub span: Span,
}

impl ChanType {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::chan_type);
        let span = Span::from_pest(pair.as_span());
        let element_type = Box::new(TypeExpr::from_pair(pair.into_inner().next().unwrap()));
        ChanType { element_type, span }
    }
}

#[derive(Debug, Clone)]
pub struct TupleType {
    pub element_types: Vec<TypeConstraint>,
    pub span: Span,
}

impl TupleType {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::tuple_type);
        let span = Span::from_pest(pair.as_span());

        let mut element_types = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::type_list {
                for tc in inner.into_inner() {
                    element_types.push(TypeConstraint::from_pair(tc));
                }
            }
        }

        TupleType {
            element_types,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub param_types: Vec<TypeExpr>,
    pub return_type: Option<Box<TypeExpr>>,
    pub span: Span,
}

impl FunctionType {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::function_type);
        let span = Span::from_pest(pair.as_span());

        let mut param_types = Vec::new();
        let mut return_type = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::param_type_list => {
                    for p in inner.into_inner() {
                        param_types.push(TypeExpr::from_pair(p));
                    }
                }
                Rule::type_expr => return_type = Some(Box::new(TypeExpr::from_pair(inner))),
                _ => {}
            }
        }

        FunctionType {
            param_types,
            return_type,
            span,
        }
    }
}
