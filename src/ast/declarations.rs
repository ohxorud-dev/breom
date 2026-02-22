use crate::Rule;
use pest::iterators::Pair;

use super::attributes::Attribute;
use super::common::{Span, Visibility};
use super::expressions::{Block, Expression};
use super::types::TypeExpr;

#[derive(Debug, Clone)]
pub struct DefineDecl {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Expression,
    pub span: Span,
}

impl DefineDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::define_decl);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut type_annotation = None;
        let mut value = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::type_annotation => {
                    type_annotation = Some(TypeExpr::from_pair(inner.into_inner().next().unwrap()))
                }
                Rule::expression => value = Some(Expression::from_pair(inner)),
                _ => {}
            }
        }

        DefineDecl {
            name,
            type_annotation,
            value: value.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub visibility: Visibility,
    pub mutable: bool,
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Expression,
    pub span: Span,
}

impl VarDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::var_decl);
        let span = Span::from_pest(pair.as_span());

        let mut visibility = Visibility::Private;
        let mut mutable = false;
        let mut name = String::new();
        let mut type_annotation = None;
        let mut value = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::mutability => mutable = true,
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::type_annotation => {
                    type_annotation = Some(TypeExpr::from_pair(inner.into_inner().next().unwrap()))
                }
                Rule::expression => value = Some(Expression::from_pair(inner)),
                _ => {}
            }
        }

        VarDecl {
            visibility,
            mutable,
            name,
            type_annotation,
            value: value.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]

pub struct FunctionDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub throws: bool,
    pub body: Block,
    pub span: Span,
}

impl FunctionDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::function_decl);
        let span = Span::from_pest(pair.as_span());

        let mut attributes = Vec::new();
        let mut visibility = Visibility::Private;
        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut throws = false;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::attributes => {
                    for attr in inner.into_inner() {
                        attributes.push(Attribute::from_pair(attr));
                    }
                }
                Rule::visibility => visibility = Visibility::Public,
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::generic_params => {
                    for gp in inner.into_inner() {
                        generic_params.push(GenericParam::from_pair(gp));
                    }
                }
                Rule::param_list => {
                    for p in inner.into_inner() {
                        params.push(Param::from_pair(p));
                    }
                }
                Rule::return_type_spec => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::throws_keyword => throws = true,
                            Rule::type_expr => return_type = Some(TypeExpr::from_pair(r)),
                            _ => {}
                        }
                    }
                }
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }

        FunctionDecl {
            attributes,
            visibility,
            name,
            generic_params,
            params,
            return_type,
            throws,
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub constraints: Vec<TypeExpr>,
    pub span: Span,
}

impl GenericParam {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::generic_param);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut constraints = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::constraint_list => {
                    for c in inner.into_inner() {
                        constraints.push(TypeExpr::from_pair(c));
                    }
                }
                _ => {}
            }
        }

        GenericParam {
            name,
            constraints,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub mutable: bool,
    pub name: String,
    pub type_expr: TypeExpr,
    pub span: Span,
}

impl Param {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::param);
        let span = Span::from_pest(pair.as_span());

        let mut mutable = false;
        let mut name = String::new();
        let mut type_expr = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::mutability => mutable = true,
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::type_expr => type_expr = Some(TypeExpr::from_pair(inner)),
                _ => {}
            }
        }

        Param {
            mutable,
            name,
            type_expr: type_expr.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub inheritance: Vec<TypeExpr>,
    pub members: Vec<StructMember>,
    pub span: Span,
}

impl StructDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::struct_decl);
        let span = Span::from_pest(pair.as_span());

        let mut attributes = Vec::new();
        let mut visibility = Visibility::Private;
        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut inheritance = Vec::new();
        let mut members = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::attributes => {
                    for attr in inner.into_inner() {
                        attributes.push(Attribute::from_pair(attr));
                    }
                }
                Rule::visibility => visibility = Visibility::Public,
                Rule::identifier | Rule::base_type => name = inner.as_str().to_string(),
                Rule::generic_params => {
                    for gp in inner.into_inner() {
                        generic_params.push(GenericParam::from_pair(gp));
                    }
                }
                Rule::inheritance => {
                    for t in inner.into_inner() {
                        inheritance.push(TypeExpr::from_pair(t));
                    }
                }
                Rule::struct_body => {
                    for m in inner.into_inner() {
                        members.push(StructMember::from_pair(m));
                    }
                }
                _ => {}
            }
        }

        StructDecl {
            attributes,
            visibility,
            name,
            generic_params,
            inheritance,
            members,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub visibility: Visibility,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub throws: bool,
    pub body: Block,
    pub span: Span,
}

impl ConstructorDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::constructor_decl);
        let span = Span::from_pest(pair.as_span());
        let mut visibility = Visibility::Private;
        let mut params = Vec::new();
        let mut return_type = None;
        let mut throws = false;
        let mut body = None;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::param_list => {
                    for p in inner.into_inner() {
                        params.push(Param::from_pair(p));
                    }
                }
                Rule::return_type_spec => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::throws_keyword => throws = true,
                            Rule::type_expr => return_type = Some(TypeExpr::from_pair(r)),
                            _ => {}
                        }
                    }
                }
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }
        ConstructorDecl {
            visibility,
            params,
            return_type,
            throws,
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DefaultDecl {
    pub visibility: Visibility,
    pub body: Block,
    pub span: Span,
}

impl DefaultDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::default_decl);
        let span = Span::from_pest(pair.as_span());
        let mut visibility = Visibility::Private;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }

        DefaultDecl {
            visibility,
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OperatorDecl {
    pub visibility: Visibility,
    pub op_symbol: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub throws: bool,
    pub body: Block,
    pub span: Span,
}

impl OperatorDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::operator_decl);
        let span = Span::from_pest(pair.as_span());
        let mut visibility = Visibility::Private;
        let mut op_symbol = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut throws = false;
        let mut body = None;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::operator_symbol => op_symbol = inner.as_str().to_string(),
                Rule::param_list => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::param {
                            params.push(Param::from_pair(p));
                        }
                    }
                }
                Rule::return_type_spec => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::throws_keyword => throws = true,
                            Rule::type_expr => return_type = Some(TypeExpr::from_pair(r)),
                            _ => {}
                        }
                    }
                }
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }
        OperatorDecl {
            visibility,
            op_symbol,
            params,
            return_type,
            throws,
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversionDecl {
    pub visibility: Visibility,
    pub target_type: TypeExpr,
    pub body: Block,
    pub span: Span,
}

impl ConversionDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::conversion_decl);
        let span = Span::from_pest(pair.as_span());
        let mut visibility = Visibility::Private;
        let mut target_type = None;
        let mut body = None;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::type_expr => target_type = Some(TypeExpr::from_pair(inner)),
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }
        ConversionDecl {
            visibility,
            target_type: target_type.unwrap(),
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StructMember {
    Field(FieldDecl),
    Method(MethodDecl),
    Constructor(ConstructorDecl),
    Default(DefaultDecl),
    Operator(OperatorDecl),
    Conversion(ConversionDecl),
}

impl StructMember {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::struct_member);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::field_decl => StructMember::Field(FieldDecl::from_pair(inner)),
            Rule::method_decl => StructMember::Method(MethodDecl::from_pair(inner)),
            Rule::constructor_decl => StructMember::Constructor(ConstructorDecl::from_pair(inner)),
            Rule::default_decl => StructMember::Default(DefaultDecl::from_pair(inner)),
            Rule::operator_decl => StructMember::Operator(OperatorDecl::from_pair(inner)),
            Rule::conversion_decl => StructMember::Conversion(ConversionDecl::from_pair(inner)),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub visibility: Visibility,
    pub is_point: bool,
    pub mutable: bool,
    pub name: String,
    pub type_expr: TypeExpr,
    pub span: Span,
}

impl FieldDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::field_decl);
        let span = Span::from_pest(pair.as_span());

        let mut visibility = Visibility::Private;
        let mut is_point = false;
        let mut mutable = false;
        let mut name = String::new();
        let mut type_expr = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::point_keyword => is_point = true,
                Rule::mutability => mutable = true,
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::type_expr => type_expr = Some(TypeExpr::from_pair(inner)),
                _ => {}
            }
        }

        FieldDecl {
            visibility,
            is_point,
            mutable,
            name,
            type_expr: type_expr.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<MethodParam>,
    pub return_type: Option<TypeExpr>,
    pub throws: bool,
    pub body: Block,
    pub span: Span,
}

impl MethodDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::method_decl);
        let span = Span::from_pest(pair.as_span());

        let mut attributes = Vec::new();
        let mut visibility = Visibility::Private;
        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut throws = false;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::attributes => {
                    for attr in inner.into_inner() {
                        attributes.push(Attribute::from_pair(attr));
                    }
                }
                Rule::visibility => visibility = Visibility::Public,
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::generic_params => {
                    for gp in inner.into_inner() {
                        generic_params.push(GenericParam::from_pair(gp));
                    }
                }
                Rule::method_param_list => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::method_param {
                            params.push(MethodParam::from_pair(p));
                        }
                    }
                }
                Rule::return_type_spec => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::throws_keyword => throws = true,
                            Rule::type_expr => return_type = Some(TypeExpr::from_pair(r)),
                            _ => {}
                        }
                    }
                }
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }

        MethodDecl {
            attributes,
            visibility,
            name,
            generic_params,
            params,
            return_type,
            throws,
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MethodParam {
    SelfParam,
    Regular(Param),
}

impl MethodParam {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::method_param);
        if pair.as_str().trim() == "self" {
            return MethodParam::SelfParam;
        }
        let inner = pair
            .into_inner()
            .next()
            .expect("method_param must have inner for param");
        MethodParam::Regular(Param::from_pair(inner))
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub visibility: Visibility,
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub members: Vec<InterfaceMember>,
    pub span: Span,
}

impl InterfaceDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::interface_decl);
        let span = Span::from_pest(pair.as_span());

        let mut visibility = Visibility::Private;
        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut members = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::identifier | Rule::base_type => name = inner.as_str().to_string(),
                Rule::generic_params => {
                    for gp in inner.into_inner() {
                        generic_params.push(GenericParam::from_pair(gp));
                    }
                }
                Rule::interface_body => {
                    for m in inner.into_inner() {
                        members.push(InterfaceMember::from_pair(m));
                    }
                }
                _ => {}
            }
        }

        InterfaceDecl {
            visibility,
            name,
            generic_params,
            members,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceMember {
    Signature(MethodSignature),
    DefaultMethod(DefaultMethod),
    ConversionSignature(InterfaceConversionSignature),
    DefaultConversion(InterfaceDefaultConversion),
}

impl InterfaceMember {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::interface_member);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::method_signature => InterfaceMember::Signature(MethodSignature::from_pair(inner)),
            Rule::default_method => InterfaceMember::DefaultMethod(DefaultMethod::from_pair(inner)),
            Rule::interface_conversion_signature => {
                InterfaceMember::ConversionSignature(InterfaceConversionSignature::from_pair(inner))
            }
            Rule::interface_conversion_default => {
                InterfaceMember::DefaultConversion(InterfaceDefaultConversion::from_pair(inner))
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceConversionSignature {
    pub target_type: TypeExpr,
    pub span: Span,
}

impl InterfaceConversionSignature {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::interface_conversion_signature);
        let span = Span::from_pest(pair.as_span());
        let target_type = pair
            .into_inner()
            .find(|p| p.as_rule() == Rule::type_expr)
            .map(TypeExpr::from_pair)
            .expect("interface conversion signature must have target type");
        InterfaceConversionSignature { target_type, span }
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceDefaultConversion {
    pub target_type: TypeExpr,
    pub body: Block,
    pub span: Span,
}

impl InterfaceDefaultConversion {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::interface_conversion_default);
        let span = Span::from_pest(pair.as_span());
        let mut target_type = None;
        let mut body = None;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::type_expr => target_type = Some(TypeExpr::from_pair(inner)),
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }
        InterfaceDefaultConversion {
            target_type: target_type.expect("interface default conversion must have target type"),
            body: body.expect("interface default conversion must have body"),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<MethodParam>,
    pub return_type: Option<TypeExpr>,
    pub throws: bool,
    pub span: Span,
}

impl MethodSignature {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::method_signature);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut throws = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::generic_params => {
                    for gp in inner.into_inner() {
                        generic_params.push(GenericParam::from_pair(gp));
                    }
                }
                Rule::method_param_list => {
                    for p in inner.into_inner() {
                        params.push(MethodParam::from_pair(p));
                    }
                }
                Rule::return_type_spec => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::throws_keyword => throws = true,
                            Rule::type_expr => return_type = Some(TypeExpr::from_pair(r)),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        MethodSignature {
            name,
            generic_params,
            params,
            return_type,
            throws,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DefaultMethod {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<MethodParam>,
    pub return_type: Option<TypeExpr>,
    pub throws: bool,
    pub body: Block,
    pub span: Span,
}

impl DefaultMethod {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::default_method);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut throws = false;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::generic_params => {
                    for gp in inner.into_inner() {
                        generic_params.push(GenericParam::from_pair(gp));
                    }
                }
                Rule::method_param_list => {
                    for p in inner.into_inner() {
                        params.push(MethodParam::from_pair(p));
                    }
                }
                Rule::return_type_spec => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::throws_keyword => throws = true,
                            Rule::type_expr => return_type = Some(TypeExpr::from_pair(r)),
                            _ => {}
                        }
                    }
                }
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }

        DefaultMethod {
            name,
            generic_params,
            params,
            return_type,
            throws,
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub visibility: Visibility,
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub variants: Vec<EnumVariantDecl>,
    pub span: Span,
}

impl EnumDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::enum_decl);
        let span = Span::from_pest(pair.as_span());

        let mut visibility = Visibility::Private;
        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut variants = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::visibility => visibility = Visibility::Public,
                Rule::identifier | Rule::base_type => name = inner.as_str().to_string(),
                Rule::generic_params => {
                    for gp in inner.into_inner() {
                        generic_params.push(GenericParam::from_pair(gp));
                    }
                }
                Rule::enum_body => {
                    for v in inner.into_inner() {
                        if v.as_rule() == Rule::enum_variant_decl {
                            variants.push(EnumVariantDecl::from_pair(v));
                        }
                    }
                }
                _ => {}
            }
        }

        EnumDecl {
            visibility,
            name,
            generic_params,
            variants,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumVariantDecl {
    pub name: String,
    pub payload_types: Vec<TypeExpr>,
    pub span: Span,
}

impl EnumVariantDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::enum_variant_decl);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut payload_types = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::type_expr => payload_types.push(TypeExpr::from_pair(inner)),
                _ => {}
            }
        }

        EnumVariantDecl {
            name,
            payload_types,
            span,
        }
    }
}
