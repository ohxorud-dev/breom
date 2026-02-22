use crate::ast::declarations::StructMember;
use crate::ast::expressions::{
    parse_char, parse_integer, parse_string, BinaryOp, Expression, Literal, PostfixOp,
};
use crate::ast::program::{Program, TopLevelItem};
use crate::ast::statements::AssignOp;
use crate::BreomParser;
use crate::Rule;
use pest::Parser;

#[test]
fn parse_integer_supports_radix_and_underscores() {
    assert_eq!(parse_integer("42"), 42);
    assert_eq!(parse_integer("0x10"), 16);
    assert_eq!(parse_integer("0o10"), 8);
    assert_eq!(parse_integer("0b1010"), 10);
    assert_eq!(parse_integer("1_000_000"), 1_000_000);
}

#[test]
fn parse_char_handles_plain_and_escape() {
    assert_eq!(parse_char("a"), 'a');
    assert_eq!(parse_char("\\n"), '\n');
    assert_eq!(parse_char("\\t"), '\t');
}

#[test]
fn parse_string_unescapes_common_sequences() {
    assert_eq!(parse_string("hello\\nworld"), "hello\nworld");
    assert_eq!(parse_string("a\\tb"), "a\tb");
    assert_eq!(parse_string("\\\"q\\\""), "\"q\"");
    assert_eq!(parse_string("\\{x\\}"), "{x}");
}

#[test]
fn binary_op_roundtrips() {
    let ops = [
        "||", "&&", "|", "^", "&", "==", "!=", "<", "<=", ">", ">=", "shl", "shr", "+", "-", "*",
        "/", "%",
    ];
    for op in ops {
        let parsed = BinaryOp::from_str(op);
        assert_eq!(parsed.as_str(), op);
    }
}

#[test]
fn assign_op_parsing_works() {
    assert_eq!(AssignOp::from_str("="), AssignOp::Assign);
    assert_eq!(AssignOp::from_str("+="), AssignOp::AddAssign);
    assert_eq!(AssignOp::from_str("-="), AssignOp::SubAssign);
    assert_eq!(AssignOp::from_str("*="), AssignOp::MulAssign);
    assert_eq!(AssignOp::from_str("/="), AssignOp::DivAssign);
    assert_eq!(AssignOp::from_str("%="), AssignOp::ModAssign);
    assert_eq!(AssignOp::from_str("&="), AssignOp::AndAssign);
    assert_eq!(AssignOp::from_str("|="), AssignOp::OrAssign);
    assert_eq!(AssignOp::from_str("^="), AssignOp::XorAssign);
}

#[test]
fn program_from_pair_parses_function() {
    let source = "fn main() Int { return 0 }";
    let mut pairs = BreomParser::parse(Rule::program, source).unwrap();
    let program = Program::from_pair(pairs.next().unwrap());

    assert!(program.depends.is_empty());
    assert_eq!(program.items.len(), 1);
}

#[test]
fn program_from_pair_parses_attribute_declaration_and_args() {
    let source = r#"
attribute bench(iterations Int, warmup Int)
@bench(10 + 2, 3 * 4)
fn main() Int { return 0 }
"#;
    let mut pairs = BreomParser::parse(Rule::program, source).unwrap();
    let program = Program::from_pair(pairs.next().unwrap());

    assert_eq!(program.items.len(), 2);

    match &program.items[0] {
        TopLevelItem::AttributeDecl(decl) => {
            assert_eq!(decl.name, "bench");
            assert_eq!(decl.params.len(), 2);
            assert_eq!(decl.params[0].name, "iterations");
            assert_eq!(decl.params[1].name, "warmup");
        }
        _ => panic!("expected attribute declaration"),
    }

    match &program.items[1] {
        TopLevelItem::Function(func) => {
            assert_eq!(func.attributes.len(), 1);
            assert_eq!(func.attributes[0].name, "bench");
            assert_eq!(func.attributes[0].args.len(), 2);
        }
        _ => panic!("expected function declaration"),
    }
}

#[test]
fn program_from_pair_parses_struct_default_member() {
    let source = r#"
struct User {
default() {
    return User { }
}
}
"#;
    let mut pairs = BreomParser::parse(Rule::program, source).unwrap();
    let program = Program::from_pair(pairs.next().unwrap());

    assert_eq!(program.items.len(), 1);
    match &program.items[0] {
        TopLevelItem::Struct(s) => {
            assert!(s
                .members
                .iter()
                .any(|m| matches!(m, StructMember::Default(_))));
        }
        _ => panic!("expected struct declaration"),
    }
}

#[test]
fn expression_from_pair_parses_default_member_access() {
    let mut pairs = BreomParser::parse(Rule::expression, "User.default()").unwrap();
    let expr = Expression::from_pair(pairs.next().unwrap());

    match expr {
        Expression::Postfix(postfix) => {
            assert_eq!(postfix.ops.len(), 2);
            match &postfix.ops[0] {
                PostfixOp::Member(name) => assert_eq!(name, "default"),
                _ => panic!("expected member access"),
            }
            assert!(matches!(postfix.ops[1], PostfixOp::Call(_)));
        }
        _ => panic!("expected postfix expression"),
    }
}

#[test]
fn parse_integer_overflow_should_not_panic() {
    let result = std::panic::catch_unwind(|| parse_integer("18446744073709551616"));
    assert!(
        result.is_ok(),
        "overflowing integer literal currently panics"
    );
}

#[test]
fn integer_literal_above_i64_max_should_not_wrap_negative() {
    let mut pairs = BreomParser::parse(Rule::literal, "9223372036854775808").unwrap();
    let literal = Literal::from_pair(pairs.next().unwrap());
    match literal {
        Literal::Integer(value, _) => {
            assert!(value >= 0, "integer literal wrapped to negative: {value}");
        }
        _ => panic!("expected integer literal"),
    }
}
