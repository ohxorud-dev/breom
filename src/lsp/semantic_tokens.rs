use pest::Parser;
use tower_lsp::lsp_types::*;

use super::analysis::offset_to_position;
use crate::{BreomParser, Rule};

pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::EVENT,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::DECORATOR,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::MODIFICATION,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ],
    }
}

const TT_TYPE: u32 = 1;
const TT_STRUCT: u32 = 5;
const TT_INTERFACE: u32 = 4;
const TT_PARAMETER: u32 = 7;
const TT_VARIABLE: u32 = 8;
const TT_FUNCTION: u32 = 12;
const TT_KEYWORD: u32 = 15;
const TT_COMMENT: u32 = 17;
const TT_STRING: u32 = 18;
const TT_NUMBER: u32 = 19;
const TT_OPERATOR: u32 = 21;

const TM_DECLARATION: u32 = 1;
const TM_DEFINITION: u32 = 2;
const TM_READONLY: u32 = 4;
const TM_STATIC: u32 = 8;

#[derive(Debug)]
struct RawToken {
    line: u32,
    start_char: u32,
    length: u32,
    token_type: u32,
    token_modifiers: u32,
}

pub fn tokenize(content: &str) -> Vec<SemanticToken> {
    let mut raw_tokens: Vec<RawToken> = Vec::new();

    add_comment_tokens(content, &mut raw_tokens);

    add_keyword_tokens(content, &mut raw_tokens);

    if let Ok(pairs) = BreomParser::parse(Rule::program, content) {
        tokenize_pairs(pairs, content, &mut raw_tokens);
    }

    raw_tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));

    let mut deduped: Vec<RawToken> = Vec::new();
    for token in raw_tokens {
        if let Some(last) = deduped.last() {
            if last.line == token.line && last.start_char + last.length > token.start_char {
                continue;
            }
        }
        deduped.push(token);
    }

    let mut result = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    for token in deduped {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start_char - prev_char
        } else {
            token.start_char
        };

        result.push(SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: token.token_modifiers,
        });

        prev_line = token.line;
        prev_char = token.start_char;
    }

    result
}

fn add_comment_tokens(content: &str, tokens: &mut Vec<RawToken>) {
    let bytes = content.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;

            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            let p = offset_to_position(content, start);
            tokens.push(RawToken {
                line: p.line,
                start_char: p.character,
                length: (i - start) as u32,
                token_type: TT_COMMENT,
                token_modifiers: 0,
            });
            continue;
        }

        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            }

            let comment_text = &content[start..i];
            let start_pos = offset_to_position(content, start);

            for (line_idx, line) in comment_text.lines().enumerate() {
                if line.is_empty() {
                    continue;
                }
                tokens.push(RawToken {
                    line: start_pos.line + line_idx as u32,
                    start_char: if line_idx == 0 {
                        start_pos.character
                    } else {
                        0
                    },
                    length: line.len() as u32,
                    token_type: TT_COMMENT,
                    token_modifiers: 0,
                });
            }
            continue;
        }

        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 1;
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }

        i += 1;
    }
}

fn tokenize_pairs<'a>(
    pairs: pest::iterators::Pairs<'a, Rule>,
    content: &str,
    tokens: &mut Vec<RawToken>,
) {
    for pair in pairs {
        let rule = pair.as_rule();
        let span = pair.as_span();
        let start = span.start();
        let end = span.end();

        let pos = offset_to_position(content, start);
        let line = pos.line;
        let start_char = pos.character;
        let length = (end - start) as u32;

        match rule {
            Rule::function_decl => {
                for inner in pair.clone().into_inner() {
                    if inner.as_rule() == Rule::identifier {
                        let s = inner.as_span();
                        let p = offset_to_position(content, s.start());
                        tokens.push(RawToken {
                            line: p.line,
                            start_char: p.character,
                            length: (s.end() - s.start()) as u32,
                            token_type: TT_FUNCTION,
                            token_modifiers: TM_DECLARATION
                                | TM_DEFINITION
                                | TM_READONLY
                                | TM_STATIC,
                        });
                        break;
                    }
                }
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
            Rule::struct_decl => {
                for inner in pair.clone().into_inner() {
                    if inner.as_rule() == Rule::identifier {
                        let s = inner.as_span();
                        let p = offset_to_position(content, s.start());
                        tokens.push(RawToken {
                            line: p.line,
                            start_char: p.character,
                            length: (s.end() - s.start()) as u32,
                            token_type: TT_STRUCT,
                            token_modifiers: TM_DECLARATION
                                | TM_DEFINITION
                                | TM_READONLY
                                | TM_STATIC,
                        });
                        break;
                    }
                }
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
            Rule::interface_decl => {
                for inner in pair.clone().into_inner() {
                    if inner.as_rule() == Rule::identifier {
                        let s = inner.as_span();
                        let p = offset_to_position(content, s.start());
                        tokens.push(RawToken {
                            line: p.line,
                            start_char: p.character,
                            length: (s.end() - s.start()) as u32,
                            token_type: TT_INTERFACE,
                            token_modifiers: TM_DECLARATION
                                | TM_DEFINITION
                                | TM_READONLY
                                | TM_STATIC,
                        });
                        break;
                    }
                }
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
            Rule::var_decl => {
                let mut is_mut = false;
                for inner in pair.clone().into_inner() {
                    if inner.as_rule() == Rule::mutability {
                        is_mut = true;
                    }
                    if inner.as_rule() == Rule::identifier {
                        let s = inner.as_span();
                        let p = offset_to_position(content, s.start());
                        let mut mods = TM_DECLARATION | TM_DEFINITION;
                        if !is_mut {
                            mods |= TM_READONLY;
                        }
                        tokens.push(RawToken {
                            line: p.line,
                            start_char: p.character,
                            length: (s.end() - s.start()) as u32,
                            token_type: TT_VARIABLE,
                            token_modifiers: mods,
                        });
                    }
                }
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
            Rule::base_type => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_TYPE,
                    token_modifiers: 0,
                });
            }
            Rule::integer_literal | Rule::float_literal => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_NUMBER,
                    token_modifiers: 0,
                });
            }
            Rule::string_literal | Rule::multiline_string_literal | Rule::char_literal => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_STRING,
                    token_modifiers: 0,
                });
            }
            Rule::fstring_literal => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length: 2,
                    token_type: TT_STRING,
                    token_modifiers: 0,
                });

                let end_pos = offset_to_position(content, end - 1);
                tokens.push(RawToken {
                    line: end_pos.line,
                    start_char: end_pos.character,
                    length: 1,
                    token_type: TT_STRING,
                    token_modifiers: 0,
                });
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
            Rule::fstring_text | Rule::fstring_escape => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_STRING,
                    token_modifiers: 0,
                });
            }
            Rule::fstring_interpolation => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length: 1,
                    token_type: TT_OPERATOR,
                    token_modifiers: 0,
                });
                let end_pos = offset_to_position(content, end - 1);
                tokens.push(RawToken {
                    line: end_pos.line,
                    start_char: end_pos.character,
                    length: 1,
                    token_type: TT_OPERATOR,
                    token_modifiers: 0,
                });
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
            Rule::bool_literal => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_KEYWORD,
                    token_modifiers: 0,
                });
            }
            Rule::param => {
                for inner in pair.clone().into_inner() {
                    if inner.as_rule() == Rule::identifier {
                        let s = inner.as_span();
                        let p = offset_to_position(content, s.start());
                        tokens.push(RawToken {
                            line: p.line,
                            start_char: p.character,
                            length: (s.end() - s.start()) as u32,
                            token_type: TT_PARAMETER,
                            token_modifiers: TM_DECLARATION | TM_DEFINITION,
                        });
                    }
                }
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
            Rule::visibility | Rule::mutability => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_KEYWORD,
                    token_modifiers: 0,
                });
            }
            Rule::identifier => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_VARIABLE,
                    token_modifiers: 0,
                });
            }
            Rule::add_op
            | Rule::mul_op
            | Rule::cmp_op
            | Rule::eq_op
            | Rule::and_op
            | Rule::or_op
            | Rule::bitand_op
            | Rule::bitor_op
            | Rule::bitxor_op
            | Rule::shift_op => {
                tokens.push(RawToken {
                    line,
                    start_char,
                    length,
                    token_type: TT_OPERATOR,
                    token_modifiers: 0,
                });
            }
            _ => {
                tokenize_pairs(pair.into_inner(), content, tokens);
            }
        }
    }
}

fn add_keyword_tokens(content: &str, tokens: &mut Vec<RawToken>) {
    let keywords = [
        "fn",
        "struct",
        "interface",
        "if",
        "else",
        "for",
        "match",
        "return",
        "defer",
        "spawn",
        "wait",
        "break",
        "continue",
        "define",
        "import",
        "pub",
        "mut",
        "point",
        "as",
        "range",
        "default",
    ];

    let is_ident_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_';

    for keyword in keywords {
        let mut search_start = 0;
        while let Some(pos) = content[search_start..].find(keyword) {
            let abs_pos = search_start + pos;
            let before_ok = abs_pos == 0 || !is_ident_char(content.as_bytes()[abs_pos - 1]);
            let after_pos = abs_pos + keyword.len();
            let after_ok =
                after_pos >= content.len() || !is_ident_char(content.as_bytes()[after_pos]);

            if before_ok && after_ok {
                let p = offset_to_position(content, abs_pos);
                tokens.push(RawToken {
                    line: p.line,
                    start_char: p.character,
                    length: keyword.len() as u32,
                    token_type: TT_KEYWORD,
                    token_modifiers: 0,
                });
            }
            search_start = abs_pos + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolutize(tokens: &[SemanticToken]) -> Vec<(u32, u32, u32, u32)> {
        let mut out = Vec::new();
        let mut line = 0u32;
        let mut ch = 0u32;
        for token in tokens {
            line += token.delta_line;
            if token.delta_line == 0 {
                ch += token.delta_start;
            } else {
                ch = token.delta_start;
            }
            out.push((line, ch, token.length, token.token_type));
        }
        out
    }

    #[test]
    fn legend_contains_expected_token_tables() {
        let legend = get_legend();
        assert!(legend.token_types.len() >= 23);
        assert!(legend.token_modifiers.len() >= 10);
    }

    #[test]
    fn tokenize_emits_comment_keyword_and_number_tokens() {
        let content = "fn main() Int {\n// comment\nreturn 123\n}";
        let tokens = tokenize(content);
        let tokens = absolutize(&tokens);

        assert!(tokens.iter().any(|(_, _, _, ty)| *ty == TT_COMMENT));
        assert!(tokens.iter().any(|(_, _, _, ty)| *ty == TT_KEYWORD));
        assert!(tokens.iter().any(|(_, _, _, ty)| *ty == TT_NUMBER));
    }

    #[test]
    fn keyword_scan_respects_identifier_boundaries() {
        let mut tokens = Vec::new();
        add_keyword_tokens("fnx fn my_fn", &mut tokens);

        let keyword_positions: Vec<u32> = tokens
            .iter()
            .filter(|t| t.token_type == TT_KEYWORD)
            .map(|t| t.start_char)
            .collect();
        assert_eq!(keyword_positions, vec![4]);
    }
}
