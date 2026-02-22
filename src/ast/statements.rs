use crate::Rule;
use pest::iterators::Pair;

use super::common::Span;
use super::declarations::VarDecl;
use super::expressions::{parse_integer, Block, Expression, Literal};

#[derive(Debug, Clone)]
pub enum Statement {
    VarDecl(VarDecl),
    Return(ReturnStmt),
    Throw(Box<Expression>, Span),
    Defer(DeferStmt),
    For(ForStmt),
    If(IfStmt),
    Match(MatchStmt),
    Spawn(SpawnStmt),
    Wait(WaitStmt),
    Break(Span),
    Continue(Span),
    Assignment(AssignmentStmt),
    Expression(Expression),
    Instead(Box<Expression>, Span),
}

impl Statement {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::statement);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::var_decl => Statement::VarDecl(VarDecl::from_pair(inner)),
            Rule::return_stmt => Statement::Return(ReturnStmt::from_pair(inner)),
            Rule::throw_stmt => {
                let span = Span::from_pest(inner.as_span());
                let expr = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::expression)
                    .map(Expression::from_pair)
                    .unwrap_or_else(|| Expression::Literal(Literal::Integer(0, span.clone())));
                Statement::Throw(Box::new(expr), span)
            }
            Rule::defer_stmt => Statement::Defer(DeferStmt::from_pair(inner)),
            Rule::for_stmt => Statement::For(ForStmt::from_pair(inner)),
            Rule::if_stmt => Statement::If(IfStmt::from_pair(inner)),
            Rule::match_stmt => Statement::Match(MatchStmt::from_pair(inner)),
            Rule::spawn_stmt => Statement::Spawn(SpawnStmt::from_pair(inner)),
            Rule::wait_stmt => Statement::Wait(WaitStmt::from_pair(inner)),
            Rule::break_stmt => Statement::Break(Span::from_pest(inner.as_span())),
            Rule::continue_stmt => Statement::Continue(Span::from_pest(inner.as_span())),
            Rule::assignment_stmt => Statement::Assignment(AssignmentStmt::from_pair(inner)),
            Rule::expression_stmt => {
                Statement::Expression(Expression::from_pair(inner.into_inner().next().unwrap()))
            }
            Rule::instead_stmt => {
                let span = Span::from_pest(inner.as_span());
                let expr = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::expression)
                    .map(Expression::from_pair)
                    .expect("instead must have an expression");
                Statement::Instead(Box::new(expr), span)
            }
            _ => unreachable!("Unexpected statement rule: {:?}", inner.as_rule()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expression>,
    pub span: Span,
}

impl ReturnStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::return_stmt);
        let span = Span::from_pest(pair.as_span());
        let value = pair
            .into_inner()
            .find(|p| p.as_rule() == Rule::expression)
            .map(Expression::from_pair);
        ReturnStmt { value, span }
    }
}

#[derive(Debug, Clone)]
pub struct DeferStmt {
    pub body: DeferBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum DeferBody {
    Block(Block),
    Expression(Expression),
}

impl DeferStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::defer_stmt);
        let span = Span::from_pest(pair.as_span());
        let inner = pair.into_inner().next().unwrap();

        let body = match inner.as_rule() {
            Rule::block => DeferBody::Block(Block::from_pair(inner)),
            Rule::expression => DeferBody::Expression(Expression::from_pair(inner)),
            _ => unreachable!(),
        };

        DeferStmt { body, span }
    }
}

#[derive(Debug, Clone)]
pub enum ForStmt {
    Infinite(Block, Span),
    Condition(Expression, Block, Span),
    Count(u64, Block, Span),
    Range(RangeFor),
}

impl ForStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::for_stmt);
        let inner = pair.into_inner().next().unwrap();
        let span = Span::from_pest(inner.as_span());

        match inner.as_rule() {
            Rule::for_infinite => {
                let block = Block::from_pair(inner.into_inner().next().unwrap());
                ForStmt::Infinite(block, span)
            }
            Rule::for_condition => {
                let mut iter = inner.into_inner();
                let cond = Expression::from_pair(iter.next().unwrap());
                let block = Block::from_pair(iter.next().unwrap());
                ForStmt::Condition(cond, block, span)
            }
            Rule::for_count => {
                let mut iter = inner.into_inner();
                let count = parse_integer(iter.next().unwrap().as_str());
                let block = Block::from_pair(iter.next().unwrap());
                ForStmt::Count(count, block, span)
            }
            Rule::for_range => ForStmt::Range(RangeFor::from_pair(inner)),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RangeFor {
    pub index_var: String,
    pub value_var: Option<String>,
    pub iterable: Expression,
    pub body: Block,
    pub span: Span,
}

impl RangeFor {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::for_range);
        let span = Span::from_pest(pair.as_span());

        let mut identifiers = Vec::new();
        let mut iterable = None;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => identifiers.push(inner.as_str().to_string()),
                Rule::expression => iterable = Some(Expression::from_pair(inner)),
                Rule::block => body = Some(Block::from_pair(inner)),
                _ => {}
            }
        }

        let (index_var, value_var) = if identifiers.len() == 2 {
            (identifiers[0].clone(), Some(identifiers[1].clone()))
        } else {
            (identifiers[0].clone(), None)
        };

        RangeFor {
            index_var,
            value_var,
            iterable: iterable.unwrap(),
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Block,
    pub else_clause: Option<ElseClause>,
    pub span: Span,
}

impl IfStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::if_stmt);
        let span = Span::from_pest(pair.as_span());

        let mut condition = None;
        let mut then_block = None;
        let mut else_clause = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::expression => condition = Some(Expression::from_pair(inner)),
                Rule::block => then_block = Some(Block::from_pair(inner)),
                Rule::else_clause => else_clause = Some(ElseClause::from_pair(inner)),
                _ => {}
            }
        }

        IfStmt {
            condition: condition.unwrap(),
            then_block: then_block.unwrap(),
            else_clause,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ElseClause {
    ElseIf(Box<IfStmt>),
    Else(Block),
}

impl ElseClause {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::else_clause);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::if_stmt => ElseClause::ElseIf(Box::new(IfStmt::from_pair(inner))),
            Rule::block => ElseClause::Else(Block::from_pair(inner)),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchStmt {
    pub expr: Expression,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

impl MatchStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::match_stmt);
        let span = Span::from_pest(pair.as_span());

        let mut expr = None;
        let mut arms = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::expression => expr = Some(Expression::from_pair(inner)),
                Rule::match_arm => arms.push(MatchArm::from_pair(inner)),
                _ => {}
            }
        }

        MatchStmt {
            expr: expr.unwrap(),
            arms,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchArmBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchArmBody {
    Block(Block),
    Expression(Expression),
}

impl MatchArm {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::match_arm);
        let span = Span::from_pest(pair.as_span());

        let mut pattern = None;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::pattern => pattern = Some(Pattern::from_pair(inner)),
                Rule::block => body = Some(MatchArmBody::Block(Block::from_pair(inner))),
                Rule::expression => {
                    body = Some(MatchArmBody::Expression(Expression::from_pair(inner)))
                }
                _ => {}
            }
        }

        MatchArm {
            pattern: pattern.unwrap(),
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard(Span),
    Binding(String, Span),
    Literal(Literal),
    Enum(EnumPattern),
}

impl Pattern {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::pattern);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::wildcard_pattern => Pattern::Wildcard(Span::from_pest(inner.as_span())),
            Rule::binding_pattern => {
                Pattern::Binding(inner.as_str().to_string(), Span::from_pest(inner.as_span()))
            }
            Rule::literal_pattern => {
                Pattern::Literal(Literal::from_pair(inner.into_inner().next().unwrap()))
            }
            Rule::enum_pattern => Pattern::Enum(EnumPattern::from_pair(inner)),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumPattern {
    pub name: String,
    pub patterns: Vec<Pattern>,
    pub span: Span,
}

impl EnumPattern {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::enum_pattern);
        let span = Span::from_pest(pair.as_span());

        let mut name = String::new();
        let mut patterns = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => name = inner.as_str().to_string(),
                Rule::pattern_list => {
                    for p in inner.into_inner() {
                        patterns.push(Pattern::from_pair(p));
                    }
                }
                _ => {}
            }
        }

        EnumPattern {
            name,
            patterns,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpawnStmt {
    pub body: SpawnBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum SpawnBody {
    Block(Block),
    Expression(Expression),
}

impl SpawnStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::spawn_stmt);
        let span = Span::from_pest(pair.as_span());
        let inner = pair.into_inner().next().unwrap();

        let body = match inner.as_rule() {
            Rule::block => SpawnBody::Block(Block::from_pair(inner)),
            Rule::expression => SpawnBody::Expression(Expression::from_pair(inner)),
            _ => unreachable!(),
        };

        SpawnStmt { body, span }
    }
}

#[derive(Debug, Clone)]
pub struct WaitStmt {
    pub arms: Vec<WaitArm>,
    pub span: Span,
}

impl WaitStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::wait_stmt);
        let span = Span::from_pest(pair.as_span());

        let mut arms = Vec::new();

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::wait_arm {
                arms.push(WaitArm::from_pair(inner))
            }
        }

        WaitStmt { arms, span }
    }
}

#[derive(Debug, Clone)]
pub struct WaitArm {
    pub receiver: WaitReceiver,
    pub body: WaitArmBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum WaitReceiver {
    Channel { var: String, channel: Expression },
    Default,
    Timeout(Expression),
}

#[derive(Debug, Clone)]
pub enum WaitArmBody {
    Block(Block),
    Expression(Expression),
}

impl WaitArm {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::wait_arm);
        let span = Span::from_pest(pair.as_span());

        let mut receiver = None;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::wait_receive => {
                    let mut var = String::new();
                    let mut channel = None;
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::identifier => var = r.as_str().to_string(),
                            Rule::expression => channel = Some(Expression::from_pair(r)),
                            _ => {}
                        }
                    }
                    receiver = Some(WaitReceiver::Channel {
                        var,
                        channel: channel.unwrap(),
                    });
                }
                Rule::wait_default => {
                    receiver = Some(WaitReceiver::Default);
                }
                Rule::wait_timeout => {
                    let timeout_expr = inner
                        .into_inner()
                        .find(|p| p.as_rule() == Rule::expression)
                        .map(Expression::from_pair)
                        .expect("timeout arm must have duration expression");
                    receiver = Some(WaitReceiver::Timeout(timeout_expr));
                }
                Rule::block => body = Some(WaitArmBody::Block(Block::from_pair(inner))),
                Rule::expression => {
                    body = Some(WaitArmBody::Expression(Expression::from_pair(inner)))
                }
                _ => {}
            }
        }

        WaitArm {
            receiver: receiver.unwrap(),
            body: body.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssignmentStmt {
    pub target: LValue,
    pub op: AssignOp,
    pub value: Expression,
    pub span: Span,
}

impl AssignmentStmt {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::assignment_stmt);
        let span = Span::from_pest(pair.as_span());

        let mut target = None;
        let mut op = AssignOp::Assign;
        let mut value = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::lvalue => target = Some(LValue::from_pair(inner)),
                Rule::assign_op => op = AssignOp::from_str(inner.as_str()),
                Rule::expression => value = Some(Expression::from_pair(inner)),
                _ => {}
            }
        }

        AssignmentStmt {
            target: target.unwrap(),
            op,
            value: value.unwrap(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LValue {
    pub base: String,
    pub accessors: Vec<Accessor>,
    pub span: Span,
}

impl LValue {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::lvalue);
        let span = Span::from_pest(pair.as_span());

        let mut base = String::new();
        let mut accessors = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => base = inner.as_str().to_string(),
                Rule::member_access => {
                    let name = inner.into_inner().next().unwrap().as_str().to_string();
                    accessors.push(Accessor::Member(name));
                }
                Rule::index_access => {
                    let expr = Expression::from_pair(inner.into_inner().next().unwrap());
                    accessors.push(Accessor::Index(expr));
                }
                _ => {}
            }
        }

        LValue {
            base,
            accessors,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Accessor {
    Member(String),
    Index(Expression),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    AndAssign,
    OrAssign,
    XorAssign,
}

impl AssignOp {
    pub fn from_str(s: &str) -> Self {
        match s {
            "=" => AssignOp::Assign,
            "+=" => AssignOp::AddAssign,
            "-=" => AssignOp::SubAssign,
            "*=" => AssignOp::MulAssign,
            "/=" => AssignOp::DivAssign,
            "%=" => AssignOp::ModAssign,
            "&=" => AssignOp::AndAssign,
            "|=" => AssignOp::OrAssign,
            "^=" => AssignOp::XorAssign,
            _ => unreachable!(),
        }
    }
}
