use crate::Rule;
use pest::iterators::Pair;

use super::common::Span;
use super::statements::Statement;
use super::types::{GenericType, TypeExpr};

#[derive(Debug, Clone)]
pub enum Expression {
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    ChannelReceive(Box<Expression>, Span),
    Postfix(PostfixExpr),
    Literal(Literal),
    Lambda(LambdaExpr),
    StructLiteral(StructLiteral),
    TupleLiteral(TupleLiteral),
    Collection(CollectionLiteral),
    Grouped(Box<Expression>, Span),
    Identifier(String, Span),
    Ternary(TernaryExpr),
    ChannelNew(ChannelNewExpr),
    New(NewExpr),
}

impl Expression {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::expression => {
                let mut iter = pair.into_inner();
                let first = iter.next().unwrap();
                let expr = Expression::from_or_expr(first);

                if let Some(then_expr) = iter.next() {
                    let else_expr = iter.next().unwrap();
                    let span = Span { start: 0, end: 0 };
                    Expression::Ternary(TernaryExpr {
                        condition: Box::new(expr),
                        then_expr: Box::new(Expression::from_pair(then_expr)),
                        else_expr: Box::new(Expression::from_pair(else_expr)),
                        span,
                    })
                } else {
                    expr
                }
            }
            _ => Expression::from_or_expr(pair),
        }
    }

    fn from_or_expr(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::or_expr => parse_binary_chain(pair, |op| op == "||"),
            Rule::and_expr => parse_binary_chain(pair, |op| op == "&&"),
            Rule::bitor_expr => parse_binary_chain(pair, |op| op == "|"),
            Rule::bitxor_expr => parse_binary_chain(pair, |op| op == "^"),
            Rule::bitand_expr => parse_binary_chain(pair, |op| op == "&"),
            Rule::eq_expr => parse_binary_chain(pair, |op| op == "==" || op == "!="),
            Rule::cmp_expr => parse_binary_chain(pair, |op| matches!(op, "<=" | ">=" | "<" | ">")),
            Rule::shift_expr => parse_binary_chain(pair, |op| op == "shl" || op == "shr"),
            Rule::add_expr => parse_binary_chain(pair, |op| op == "+" || op == "-"),
            Rule::mul_expr => parse_binary_chain(pair, |op| matches!(op, "*" | "/" | "%")),
            Rule::unary_expr => Expression::from_unary(pair),
            Rule::postfix_expr => Expression::from_postfix(pair),
            Rule::primary_expr => Expression::from_primary(pair),
            _ => Expression::from_or_expr(pair.into_inner().next().unwrap()),
        }
    }

    fn from_unary(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::unary_expr);
        let span = Span::from_pest(pair.as_span());
        let mut inner = pair.into_inner();
        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::channel_receive => {
                let expr = Expression::from_postfix(first.into_inner().next().unwrap());
                Expression::ChannelReceive(Box::new(expr), span)
            }
            Rule::not_expr => {
                let operand = Expression::from_unary(first.into_inner().next().unwrap());
                Expression::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            Rule::neg_expr => {
                let operand = Expression::from_unary(first.into_inner().next().unwrap());
                Expression::Unary(UnaryExpr {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            Rule::bitnot_expr => {
                let operand = Expression::from_unary(first.into_inner().next().unwrap());
                Expression::Unary(UnaryExpr {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                    span,
                })
            }
            Rule::postfix_expr => Expression::from_postfix(first),
            _ => unreachable!("Unexpected unary expr rule: {:?}", first.as_rule()),
        }
    }

    fn from_postfix(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::postfix_expr);
        let span = Span::from_pest(pair.as_span());
        let mut inner = pair.into_inner();

        let primary = Expression::from_primary(inner.next().unwrap());
        let ops: Vec<_> = inner.collect();

        if ops.is_empty() {
            return primary;
        }

        let postfix_ops: Vec<PostfixOp> = ops
            .into_iter()
            .map(|op| {
                let inner_op = if op.as_rule() == Rule::postfix_op {
                    op.into_inner().next().unwrap()
                } else {
                    op
                };

                match inner_op.as_rule() {
                    Rule::call_op => {
                        let args = inner_op
                            .into_inner()
                            .next()
                            .map(|arg_list| {
                                arg_list.into_inner().map(Expression::from_pair).collect()
                            })
                            .unwrap_or_default();
                        PostfixOp::Call(args)
                    }
                    Rule::member_access => {
                        let name = inner_op.into_inner().next().unwrap().as_str().to_string();
                        PostfixOp::Member(name)
                    }
                    Rule::index_access => {
                        let expr = Expression::from_pair(inner_op.into_inner().next().unwrap());
                        PostfixOp::Index(Box::new(expr))
                    }
                    Rule::type_cast => {
                        let ty = TypeExpr::from_pair(inner_op.into_inner().next().unwrap());
                        PostfixOp::Cast(ty)
                    }
                    Rule::error_propagation => PostfixOp::ErrorProp,
                    Rule::channel_send => {
                        let expr = Expression::from_pair(inner_op.into_inner().next().unwrap());
                        PostfixOp::ChannelSend(Box::new(expr))
                    }
                    Rule::catch_block => {
                        let block = inner_op
                            .into_inner()
                            .find(|p| p.as_rule() == Rule::block)
                            .map(Block::from_pair)
                            .unwrap_or_else(|| Block {
                                statements: vec![],
                                span: Span { start: 0, end: 0 },
                            });
                        PostfixOp::Catch(block)
                    }
                    Rule::instead_fallback => {
                        let expr = Expression::from_pair(inner_op.into_inner().next().unwrap());
                        PostfixOp::Instead(Box::new(expr))
                    }
                    _ => unreachable!("Unexpected postfix op: {:?}", inner_op.as_rule()),
                }
            })
            .collect();

        Expression::Postfix(PostfixExpr {
            base: Box::new(primary),
            ops: postfix_ops,
            span,
        })
    }

    fn from_primary(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::primary_expr);
        let inner = pair.into_inner().next().unwrap();
        let span = Span::from_pest(inner.as_span());
        match inner.as_rule() {
            Rule::literal => Expression::Literal(Literal::from_pair(inner)),
            Rule::lambda_expr => Expression::Lambda(LambdaExpr::from_pair(inner)),
            Rule::struct_literal => Expression::StructLiteral(StructLiteral::from_pair(inner)),
            Rule::tuple_literal => Expression::TupleLiteral(TupleLiteral::from_pair(inner)),
            Rule::collection_literal => Expression::Collection(CollectionLiteral::from_pair(inner)),
            Rule::grouped_expr => {
                let expr = Expression::from_pair(inner.into_inner().next().unwrap());
                Expression::Grouped(Box::new(expr), span)
            }
            Rule::identifier => Expression::Identifier(inner.as_str().to_string(), span),
            Rule::channel_new_expr => Expression::ChannelNew(ChannelNewExpr::from_pair(inner)),
            Rule::new_expr => Expression::New(NewExpr::from_pair(inner)),
            _ => unreachable!("Unexpected primary: {:?}", inner.as_rule()),
        }
    }
}

fn parse_binary_chain<F>(pair: Pair<Rule>, _is_op: F) -> Expression
where
    F: Fn(&str) -> bool,
{
    let span = Span::from_pest(pair.as_span());
    let mut inner = pair.into_inner();

    let first = inner.next().unwrap();
    let mut left = Expression::from_or_expr(first);

    while let Some(op_or_expr) = inner.next() {
        let op_str = op_or_expr.as_str();
        let op = BinaryOp::from_str(op_str);

        if let Some(right_pair) = inner.next() {
            let right = Expression::from_or_expr(right_pair);
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span: span.clone(),
            });
        }
    }

    left
}

#[derive(Debug, Clone)]
pub struct TernaryExpr {
    pub condition: Box<Expression>,
    pub then_expr: Box<Expression>,
    pub else_expr: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub op: BinaryOp,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Or,
    And,
    BitOr,
    BitXor,
    BitAnd,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl BinaryOp {
    pub fn from_str(s: &str) -> Self {
        match s {
            "||" => BinaryOp::Or,
            "&&" => BinaryOp::And,
            "|" => BinaryOp::BitOr,
            "^" => BinaryOp::BitXor,
            "&" => BinaryOp::BitAnd,
            "==" => BinaryOp::Eq,
            "!=" => BinaryOp::Ne,
            "<" => BinaryOp::Lt,
            "<=" => BinaryOp::Le,
            ">" => BinaryOp::Gt,
            ">=" => BinaryOp::Ge,
            "shl" => BinaryOp::Shl,
            "shr" => BinaryOp::Shr,
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Mod,
            _ => unreachable!("Unknown binary op: {}", s),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::BitAnd => "&",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::Shl => "shl",
            BinaryOp::Shr => "shr",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
    BitNot,
}

#[derive(Debug, Clone)]
pub struct PostfixExpr {
    pub base: Box<Expression>,
    pub ops: Vec<PostfixOp>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum PostfixOp {
    Call(Vec<Expression>),
    Member(String),
    Index(Box<Expression>),
    Cast(TypeExpr),
    ErrorProp,
    ChannelSend(Box<Expression>),
    Catch(Block),
    Instead(Box<Expression>),
}

#[derive(Debug, Clone)]
pub struct LambdaExpr {
    pub params: Vec<LambdaParam>,
    pub return_type: Option<TypeExpr>,
    pub body: LambdaBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum LambdaBody {
    Block(Block),
    Expression(Box<Expression>),
}

impl LambdaExpr {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::lambda_expr);
        let span = Span::from_pest(pair.as_span());

        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::lambda_param_list => {
                    for p in inner.into_inner() {
                        params.push(LambdaParam::from_pair(p));
                    }
                }
                Rule::type_expr => return_type = Some(TypeExpr::from_pair(inner)),
                Rule::block => body = Some(LambdaBody::Block(Block::from_pair(inner))),
                Rule::expression => {
                    body = Some(LambdaBody::Expression(Box::new(Expression::from_pair(
                        inner,
                    ))))
                }
                _ => {}
            }
        }

        LambdaExpr {
            params,
            return_type,
            body: body.unwrap(),
            span,
        }
    }
}

impl LambdaParam {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::lambda_param);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut type_annotation = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::type_expr => type_annotation = Some(TypeExpr::from_pair(inner)),
                _ => {}
            }
        }

        LambdaParam {
            name,
            type_annotation,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructLiteral {
    pub type_expr: TypeExpr,
    pub fields: Vec<FieldInit>,
    pub span: Span,
}

impl StructLiteral {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::struct_literal);
        let span = Span::from_pest(pair.as_span());

        let mut type_expr = None;
        let mut fields = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::type_expr => type_expr = Some(TypeExpr::from_pair(inner)),
                Rule::field_init_list => {
                    for f in inner.into_inner() {
                        if f.as_rule() == Rule::field_init {
                            fields.push(FieldInit::from_pair(f));
                        }
                    }
                }
                _ => {}
            }
        }

        StructLiteral {
            type_expr: type_expr.unwrap(),
            fields,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: Expression,
    pub span: Span,
}

impl FieldInit {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::field_init);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut value = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::expression => value = Some(Expression::from_pair(inner)),
                _ => {}
            }
        }

        FieldInit {
            name,
            value: value.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelNewExpr {
    pub element_type: TypeExpr,
    pub args: Vec<Expression>,
    pub span: Span,
}

impl ChannelNewExpr {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::channel_new_expr);
        let span = Span::from_pest(pair.as_span());

        let mut element_type = None;
        let mut args = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::type_expr => element_type = Some(TypeExpr::from_pair(inner)),
                Rule::arg_list => {
                    for arg in inner.into_inner() {
                        args.push(Expression::from_pair(arg));
                    }
                }
                _ => {}
            }
        }

        ChannelNewExpr {
            element_type: element_type.unwrap(),
            args,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewExpr {
    pub type_name: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

impl NewExpr {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::new_expr);
        let span = Span::from_pest(pair.as_span());
        let inner = pair.into_inner();

        let mut type_name = String::new();
        let mut args = Vec::new();

        for p in inner {
            match p.as_rule() {
                Rule::base_type => type_name = p.as_str().to_string(),
                Rule::generic_type => {
                    let generic = GenericType::from_pair(p);
                    type_name = generic.base;
                }
                Rule::arg_list => {
                    for arg in p.into_inner() {
                        args.push(Expression::from_pair(arg));
                    }
                }
                _ => {}
            }
        }

        NewExpr {
            type_name,
            args,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TupleLiteral {
    pub elements: Vec<Expression>,
    pub span: Span,
}

impl TupleLiteral {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::tuple_literal);
        let span = Span::from_pest(pair.as_span());

        let mut elements = Vec::new();
        for inner in pair.into_inner() {
            elements.push(Expression::from_pair(inner));
        }

        TupleLiteral { elements, span }
    }
}

#[derive(Debug, Clone)]
pub enum CollectionLiteral {
    DynamicArray(Vec<Expression>, Span),
    RepeatedArray {
        value: Box<Expression>,
        count: Box<Expression>,
        span: Span,
    },
    Map(Vec<(Expression, Expression)>, Span),
    Set(Vec<Expression>, Span),
}

impl CollectionLiteral {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::collection_literal);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::array_literal => {
                let span = Span::from_pest(inner.as_span());
                let mut parts = inner.into_inner();
                if let Some(content) = parts.next() {
                    match content.as_rule() {
                        Rule::array_elements => {
                            let elements: Vec<_> =
                                content.into_inner().map(Expression::from_pair).collect();
                            CollectionLiteral::DynamicArray(elements, span)
                        }
                        Rule::array_repeat => {
                            let mut repeat_parts = content.into_inner();
                            let value = Expression::from_pair(repeat_parts.next().unwrap());
                            let count = Expression::from_pair(repeat_parts.next().unwrap());
                            CollectionLiteral::RepeatedArray {
                                value: Box::new(value),
                                count: Box::new(count),
                                span,
                            }
                        }
                        _ => unreachable!(),
                    }
                } else {
                    CollectionLiteral::DynamicArray(Vec::new(), span)
                }
            }
            Rule::map_literal => {
                let span = Span::from_pest(inner.as_span());
                let entries: Vec<_> = inner
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::map_entry)
                    .map(|entry| {
                        let mut iter = entry.into_inner();
                        let key = Expression::from_pair(iter.next().unwrap());
                        let value = Expression::from_pair(iter.next().unwrap());
                        (key, value)
                    })
                    .collect();
                CollectionLiteral::Map(entries, span)
            }
            Rule::set_literal => {
                let span = Span::from_pest(inner.as_span());
                let elements: Vec<_> = inner.into_inner().map(Expression::from_pair).collect();
                CollectionLiteral::Set(elements, span)
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64, Span),
    Float(f64, Span),
    Bool(bool, Span),
    Char(char, Span),
    String(String, Span),
    MultilineString(String, Span),
    FString(FStringLiteral),
    Void(Span),
}

impl Literal {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let inner = match pair.as_rule() {
            Rule::literal => pair.into_inner().next().unwrap(),
            _ => pair,
        };
        let span = Span::from_pest(inner.as_span());

        match inner.as_rule() {
            Rule::integer_literal => {
                let value = i64::try_from(parse_integer(inner.as_str())).unwrap_or(i64::MAX);
                Literal::Integer(value, span)
            }
            Rule::float_literal => {
                let value: f64 = inner.as_str().parse().unwrap();
                Literal::Float(value, span)
            }
            Rule::bool_literal => {
                let value = inner.as_str() == "true";
                Literal::Bool(value, span)
            }
            Rule::char_literal => {
                let s = inner.as_str();
                let c = parse_char(&s[1..s.len() - 1]);
                Literal::Char(c, span)
            }
            Rule::string_literal => {
                let s = inner.as_str();
                let content = parse_string(&s[1..s.len() - 1]);
                Literal::String(content, span)
            }
            Rule::multiline_string_literal => {
                let s = inner.as_str();
                let content = s[3..s.len() - 3].to_string();
                Literal::MultilineString(content, span)
            }
            Rule::fstring_literal => Literal::FString(FStringLiteral::from_pair(inner)),
            Rule::void_literal => Literal::Void(span),
            _ => unreachable!("Unexpected literal: {:?}", inner.as_rule()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FStringLiteral {
    pub parts: Vec<FStringPart>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Text(String),
    Interpolation(Expression),
}

impl FStringLiteral {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::fstring_literal);
        let span = Span::from_pest(pair.as_span());

        let mut parts = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::fstring_content {
                let content = inner.into_inner().next().unwrap();
                match content.as_rule() {
                    Rule::fstring_text => {
                        parts.push(FStringPart::Text(content.as_str().to_string()));
                    }
                    Rule::fstring_escape => {
                        parts.push(FStringPart::Text(parse_escape(content.as_str())));
                    }
                    Rule::fstring_interpolation => {
                        let expr = Expression::from_pair(content.into_inner().next().unwrap());
                        parts.push(FStringPart::Interpolation(expr));
                    }
                    _ => {}
                }
            }
        }

        FStringLiteral { parts, span }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

impl Block {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::block);
        let span = Span::from_pest(pair.as_span());

        let statements = pair.into_inner().map(Statement::from_pair).collect();

        Block { statements, span }
    }
}

pub(crate) fn parse_integer(s: &str) -> u64 {
    let s = s.replace('_', "");
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).unwrap_or(u64::MAX)
    } else if s.starts_with("0o") || s.starts_with("0O") {
        u64::from_str_radix(&s[2..], 8).unwrap_or(u64::MAX)
    } else if s.starts_with("0b") || s.starts_with("0B") {
        u64::from_str_radix(&s[2..], 2).unwrap_or(u64::MAX)
    } else {
        s.parse().unwrap_or(u64::MAX)
    }
}

pub(crate) fn parse_char(s: &str) -> char {
    if s.starts_with('\\') {
        parse_escape(s).chars().next().unwrap()
    } else {
        s.chars().next().unwrap()
    }
}

pub(crate) fn parse_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                let escaped = match next {
                    'n' => {
                        chars.next();
                        '\n'
                    }
                    'r' => {
                        chars.next();
                        '\r'
                    }
                    't' => {
                        chars.next();
                        '\t'
                    }
                    '\\' => {
                        chars.next();
                        '\\'
                    }
                    '"' => {
                        chars.next();
                        '"'
                    }
                    '\'' => {
                        chars.next();
                        '\''
                    }
                    '0' => {
                        chars.next();
                        '\0'
                    }
                    '{' => {
                        chars.next();
                        '{'
                    }
                    '}' => {
                        chars.next();
                        '}'
                    }
                    _ => c,
                };
                result.push(escaped);
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn parse_escape(s: &str) -> String {
    match s {
        "\\n" => "\n".to_string(),
        "\\r" => "\r".to_string(),
        "\\t" => "\t".to_string(),
        "\\\\" => "\\".to_string(),
        "\\\"" => "\"".to_string(),
        "\\'" => "'".to_string(),
        "\\0" => "\0".to_string(),
        "\\{" => "{".to_string(),
        "\\}" => "}".to_string(),
        _ => s.to_string(),
    }
}
