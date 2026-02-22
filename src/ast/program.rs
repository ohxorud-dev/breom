use crate::Rule;
use pest::iterators::Pair;

use super::attributes::AttributeDecl;
use super::common::Span;
use super::declarations::{DefineDecl, EnumDecl, FunctionDecl, InterfaceDecl, StructDecl};
use super::statements::Statement;

#[derive(Debug, Clone)]
pub struct Program {
    pub module: Option<ModuleDecl>,
    pub entrypoint: Option<EntrypointDecl>,
    pub depends: Vec<DependDecl>,
    pub items: Vec<TopLevelItem>,
    pub span: Span,
}

impl Program {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::program);
        let span = Span::from_pest(pair.as_span());

        let mut module = None;
        let mut entrypoint = None;
        let mut depends = Vec::new();
        let mut items = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::module_decl => module = Some(ModuleDecl::from_pair(inner)),
                Rule::entrypoint_decl => entrypoint = Some(EntrypointDecl::from_pair(inner)),
                Rule::depend_decl => depends.push(DependDecl::from_pair(inner)),
                Rule::top_level_item => items.push(TopLevelItem::from_pair(inner)),
                Rule::EOI => {}
                _ => {}
            }
        }

        Program {
            module,
            entrypoint,
            depends,
            items,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub path: ModulePath,
    pub span: Span,
}

impl ModuleDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::module_decl);
        let span = Span::from_pest(pair.as_span());
        let path = ModulePath::from_pair(pair.into_inner().next().unwrap());
        ModuleDecl { path, span }
    }
}

#[derive(Debug, Clone)]
pub struct DependDecl {
    pub path: ModulePath,
    pub alias: Option<String>,
    pub span: Span,
}

impl DependDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::depend_decl);
        let span = Span::from_pest(pair.as_span());

        let mut path = None;
        let mut alias = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::module_path => path = Some(ModulePath::from_pair(inner)),
                Rule::identifier => alias = Some(inner.as_str().to_string()),
                _ => {}
            }
        }

        DependDecl {
            path: path.unwrap(),
            alias,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntrypointDecl {
    pub path: String,
    pub span: Span,
}

impl EntrypointDecl {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::entrypoint_decl);
        let span = Span::from_pest(pair.as_span());
        let path = pair.into_inner().next().unwrap().as_str().to_string();
        EntrypointDecl { path, span }
    }
}

#[derive(Debug, Clone)]
pub struct ModulePath {
    pub segments: Vec<String>,
    pub span: Span,
}

impl ModulePath {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::module_path);
        let span = Span::from_pest(pair.as_span());
        let segments = pair.as_str().split('.').map(String::from).collect();
        ModulePath { segments, span }
    }
}

#[derive(Debug, Clone)]
pub enum TopLevelItem {
    Define(DefineDecl),
    Struct(StructDecl),
    Interface(InterfaceDecl),
    Enum(EnumDecl),
    AttributeDecl(AttributeDecl),
    Function(FunctionDecl),
    Statement(Statement),
}

impl TopLevelItem {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::top_level_item);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::define_decl => TopLevelItem::Define(DefineDecl::from_pair(inner)),
            Rule::struct_decl => TopLevelItem::Struct(StructDecl::from_pair(inner)),
            Rule::interface_decl => TopLevelItem::Interface(InterfaceDecl::from_pair(inner)),
            Rule::enum_decl => TopLevelItem::Enum(EnumDecl::from_pair(inner)),
            Rule::attribute_decl => TopLevelItem::AttributeDecl(AttributeDecl::from_pair(inner)),
            Rule::function_decl => TopLevelItem::Function(FunctionDecl::from_pair(inner)),
            Rule::statement => TopLevelItem::Statement(Statement::from_pair(inner)),
            _ => unreachable!("Unexpected rule: {:?}", inner.as_rule()),
        }
    }
}
