use pest::Parser;
use tower_lsp::lsp_types::*;

use crate::ast::common::Span as AstSpan;
use crate::ast::program::Program;
use crate::{BreomParser, Rule};

#[derive(Clone, Copy)]
struct BuiltinTypeMeta {
    name: &'static str,
    detail: &'static str,
    hover_signature: &'static str,
    hover_description: &'static str,
}

#[derive(Clone, Copy)]
struct BuiltinFunctionMeta {
    name: &'static str,
    signature: &'static str,
    description: &'static str,
}

const BUILTIN_TYPES: [BuiltinTypeMeta; 16] = [
    BuiltinTypeMeta {
        name: "Int",
        detail: "Platform-dependent integer",
        hover_signature: "type Int",
        hover_description: "Platform-dependent signed integer",
    },
    BuiltinTypeMeta {
        name: "Int8",
        detail: "8-bit signed integer",
        hover_signature: "type Int8",
        hover_description: "8-bit signed integer",
    },
    BuiltinTypeMeta {
        name: "Int16",
        detail: "16-bit signed integer",
        hover_signature: "type Int16",
        hover_description: "16-bit signed integer",
    },
    BuiltinTypeMeta {
        name: "Int32",
        detail: "32-bit signed integer",
        hover_signature: "type Int32",
        hover_description: "32-bit signed integer",
    },
    BuiltinTypeMeta {
        name: "Int64",
        detail: "64-bit signed integer",
        hover_signature: "type Int64",
        hover_description: "64-bit signed integer",
    },
    BuiltinTypeMeta {
        name: "UInt",
        detail: "Platform-dependent unsigned integer",
        hover_signature: "type UInt",
        hover_description: "Platform-dependent unsigned integer",
    },
    BuiltinTypeMeta {
        name: "UInt8",
        detail: "8-bit unsigned integer",
        hover_signature: "type UInt8",
        hover_description: "8-bit unsigned integer",
    },
    BuiltinTypeMeta {
        name: "UInt16",
        detail: "16-bit unsigned integer",
        hover_signature: "type UInt16",
        hover_description: "16-bit unsigned integer",
    },
    BuiltinTypeMeta {
        name: "UInt32",
        detail: "32-bit unsigned integer",
        hover_signature: "type UInt32",
        hover_description: "32-bit unsigned integer",
    },
    BuiltinTypeMeta {
        name: "UInt64",
        detail: "64-bit unsigned integer",
        hover_signature: "type UInt64",
        hover_description: "64-bit unsigned integer",
    },
    BuiltinTypeMeta {
        name: "Float",
        detail: "64-bit floating point",
        hover_signature: "type Float",
        hover_description: "64-bit floating point",
    },
    BuiltinTypeMeta {
        name: "Float32",
        detail: "32-bit floating point",
        hover_signature: "type Float32",
        hover_description: "32-bit floating point",
    },
    BuiltinTypeMeta {
        name: "Bool",
        detail: "Boolean type",
        hover_signature: "type Bool",
        hover_description: "Boolean type (true/false)",
    },
    BuiltinTypeMeta {
        name: "String",
        detail: "UTF-8 string",
        hover_signature: "type String",
        hover_description: "UTF-8 encoded string",
    },
    BuiltinTypeMeta {
        name: "Char",
        detail: "Unicode character",
        hover_signature: "type Char",
        hover_description: "Unicode scalar value (4 bytes)",
    },
    BuiltinTypeMeta {
        name: "Byte",
        detail: "Alias for UInt8",
        hover_signature: "type Byte",
        hover_description: "Alias for UInt8",
    },
];

const BUILTIN_FUNCTIONS: [BuiltinFunctionMeta; 4] = [
    BuiltinFunctionMeta {
        name: "print",
        signature: "fn print(v Any)",
        description: "Print without newline",
    },
    BuiltinFunctionMeta {
        name: "println",
        signature: "fn println(v Any)",
        description: "Print with newline",
    },
    BuiltinFunctionMeta {
        name: "panic",
        signature: "fn panic(msg String)",
        description: "Terminate with error",
    },
    BuiltinFunctionMeta {
        name: "assert",
        signature: "fn assert(cond Bool, msg String)",
        description: "Debug assertion",
    },
];

const KEYWORD_HOVERS: [(&str, &str); 11] = [
    ("fn", "**fn** - Function declaration keyword"),
    ("struct", "**struct** - Structure type declaration"),
    ("interface", "**interface** - Interface (trait) declaration"),
    ("if", "**if** - Conditional statement"),
    (
        "for",
        "**for** - Loop statement (condition, range, count, infinite)",
    ),
    ("match", "**match** - Pattern matching expression"),
    ("spawn", "**spawn** - Create green thread"),
    ("wait", "**wait** - Wait on multiple channels"),
    ("defer", "**defer** - Execute at function exit (LIFO)"),
    ("mut", "**mut** - Mutable variable modifier"),
    ("pub", "**pub** - Public visibility modifier"),
];

pub fn parse_and_collect_diagnostics(
    content: &str,
    _uri: &Url,
) -> (Option<Program>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    match BreomParser::parse(Rule::program, content) {
        Ok(pairs) => {
            if let Some(pair) = pairs.into_iter().next() {
                let program = Program::from_pair(pair);
                (Some(program), diagnostics)
            } else {
                diagnostics.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: "Empty program".to_string(),
                    source: Some("breom".to_string()),
                    ..Default::default()
                });
                (None, diagnostics)
            }
        }
        Err(e) => {
            let (line, col) = match e.line_col {
                pest::error::LineColLocation::Pos((l, c)) => (l, c),
                pest::error::LineColLocation::Span((l, c), _) => (l, c),
            };

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: (line - 1) as u32,
                        character: (col - 1) as u32,
                    },
                    end: Position {
                        line: (line - 1) as u32,
                        character: col as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!("{}", e),
                source: Some("breom".to_string()),
                ..Default::default()
            });
            (None, diagnostics)
        }
    }
}

pub fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;

    for (i, ch) in content.chars().enumerate() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    Position {
        line,
        character: col,
    }
}

pub fn ast_span_to_range(content: &str, span: &AstSpan) -> Range {
    Range {
        start: offset_to_position(content, span.start),
        end: offset_to_position(content, span.end),
    }
}

pub fn position_to_offset(content: &str, position: Position) -> usize {
    let mut current_line = 0u32;
    let mut current_col = 0u32;

    for (i, ch) in content.chars().enumerate() {
        if current_line == position.line && current_col == position.character {
            return i;
        }
        if ch == '\n' {
            if current_line == position.line {
                return i;
            }
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
    }

    content.len()
}

pub fn get_word_at_position(content: &str, position: Position) -> Option<(String, Range)> {
    let offset = position_to_offset(content, position);
    let bytes = content.as_bytes();

    if offset >= bytes.len() {
        return None;
    }

    let is_ident_char = |c: u8| c.is_ascii_alphanumeric() || c == b'_';

    if !is_ident_char(bytes[offset]) {
        return None;
    }

    let mut start = offset;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = offset;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }

    let word = content[start..end].to_string();
    let range = Range {
        start: offset_to_position(content, start),
        end: offset_to_position(content, end),
    };

    Some((word, range))
}

pub fn get_keywords() -> Vec<CompletionItem> {
    let keywords = [
        ("fn", "Function declaration", CompletionItemKind::KEYWORD),
        ("struct", "Struct declaration", CompletionItemKind::KEYWORD),
        (
            "interface",
            "Interface declaration",
            CompletionItemKind::KEYWORD,
        ),
        ("if", "If statement", CompletionItemKind::KEYWORD),
        ("else", "Else clause", CompletionItemKind::KEYWORD),
        ("for", "For loop", CompletionItemKind::KEYWORD),
        ("match", "Match expression", CompletionItemKind::KEYWORD),
        ("return", "Return statement", CompletionItemKind::KEYWORD),
        ("mut", "Mutable modifier", CompletionItemKind::KEYWORD),
        ("pub", "Public access modifier", CompletionItemKind::KEYWORD),
        (
            "spawn",
            "Spawn new green thread",
            CompletionItemKind::KEYWORD,
        ),
        ("wait", "Wait on channels", CompletionItemKind::KEYWORD),
        ("defer", "Defer statement", CompletionItemKind::KEYWORD),
        (
            "define",
            "Compile-time constant",
            CompletionItemKind::KEYWORD,
        ),
        ("import", "Import dependency", CompletionItemKind::KEYWORD),
        ("true", "Boolean true", CompletionItemKind::KEYWORD),
        ("false", "Boolean false", CompletionItemKind::KEYWORD),
    ];

    keywords
        .iter()
        .map(|(name, detail, kind)| CompletionItem {
            label: name.to_string(),
            kind: Some(*kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

pub fn get_hover_markdown(word: &str) -> Option<String> {
    for builtin in &BUILTIN_TYPES {
        if builtin.name == word {
            return Some(format!(
                "```breom\n{}\n```\n{}",
                builtin.hover_signature, builtin.hover_description
            ));
        }
    }

    for builtin in &BUILTIN_FUNCTIONS {
        if builtin.name == word {
            return Some(format!(
                "```breom\n{}\n```\n{}",
                builtin.signature, builtin.description
            ));
        }
    }

    for (keyword, help) in KEYWORD_HOVERS {
        if keyword == word {
            return Some(help.to_string());
        }
    }

    if word == "return" {
        return Some("**return** - Return from function".to_string());
    }

    None
}

pub fn is_import_context(content: &str, position: Position) -> bool {
    let line_idx = position.line as usize;
    let Some(line) = content.lines().nth(line_idx) else {
        return false;
    };
    let prefix_end = (position.character as usize).min(line.len());
    let prefix = &line[..prefix_end];
    prefix.trim_start().starts_with("import ")
}

pub fn get_builtin_types() -> Vec<CompletionItem> {
    BUILTIN_TYPES
        .iter()
        .map(|builtin| CompletionItem {
            label: builtin.name.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some(builtin.detail.to_string()),
            ..Default::default()
        })
        .collect()
}

pub fn get_builtin_functions() -> Vec<CompletionItem> {
    BUILTIN_FUNCTIONS
        .iter()
        .map(|builtin| CompletionItem {
            label: builtin.name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(builtin.signature.to_string()),
            documentation: Some(Documentation::String(builtin.description.to_string())),
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_collect_diagnostics_reports_valid_program() {
        let uri = Url::parse("file:///tmp/ok.brm").unwrap();
        let (program, diagnostics) =
            parse_and_collect_diagnostics("fn main() Int { return 0 }", &uri);
        assert!(program.is_some());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn parse_and_collect_diagnostics_reports_syntax_error() {
        let uri = Url::parse("file:///tmp/bad.brm").unwrap();
        let (program, diagnostics) = parse_and_collect_diagnostics("fn main( {", &uri);
        assert!(program.is_none());
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn offset_and_position_convert_back_and_forth() {
        let content = "abc\ndef";
        let pos = offset_to_position(content, 5);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 1);
        assert_eq!(position_to_offset(content, pos), 5);
    }

    #[test]
    fn get_word_at_position_extracts_identifier() {
        let content = "val := test_value";
        let at_v = Position {
            line: 0,
            character: 9,
        };
        let (word, range) = get_word_at_position(content, at_v).unwrap();

        assert_eq!(word, "test_value");
        assert_eq!(range.start.character, 7);
        assert_eq!(range.end.character, 17);
    }

    #[test]
    fn builtins_and_keywords_include_core_entries() {
        let keywords = get_keywords();
        assert!(keywords.iter().any(|k| k.label == "fn"));
        assert!(keywords.iter().any(|k| k.label == "match"));

        let types = get_builtin_types();
        assert!(types.iter().any(|t| t.label == "Int"));
        assert!(types.iter().any(|t| t.label == "String"));

        let funcs = get_builtin_functions();
        assert!(funcs.iter().any(|f| f.label == "println"));

        let hover = get_hover_markdown("Int").unwrap();
        assert!(hover.contains("type Int"));
    }

    #[test]
    fn import_context_detected_from_prefix() {
        let content = "import net.http\nfn main() Int { return 0 }\n";
        assert!(is_import_context(
            content,
            Position {
                line: 0,
                character: 7
            }
        ));
        assert!(!is_import_context(
            content,
            Position {
                line: 1,
                character: 3
            }
        ));
    }
}
