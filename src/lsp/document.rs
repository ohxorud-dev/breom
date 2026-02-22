use dashmap::DashMap;
use tower_lsp::lsp_types::{TextDocumentContentChangeEvent, Url};

use crate::ast::program::Program;
use crate::lsp::analysis::position_to_offset;

#[derive(Debug)]
pub struct Document {
    pub content: String,
    pub program: Option<Program>,
    pub version: i32,
}

impl Document {
    pub fn new(content: String, version: i32) -> Self {
        Self {
            content,
            program: None,
            version,
        }
    }
}

#[derive(Debug, Default)]
pub struct DocumentCache {
    documents: DashMap<Url, Document>,
}

impl DocumentCache {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    pub fn open(&self, url: Url, content: String, version: i32) {
        self.documents.insert(url, Document::new(content, version));
    }

    #[allow(dead_code)]
    pub fn update(&self, url: &Url, content: String, version: i32) {
        if let Some(mut doc) = self.documents.get_mut(url) {
            doc.content = content;
            doc.version = version;
            doc.program = None;
        }
    }

    pub fn apply_changes(
        &self,
        url: &Url,
        changes: Vec<TextDocumentContentChangeEvent>,
        version: i32,
    ) -> bool {
        let Some(mut doc) = self.documents.get_mut(url) else {
            return false;
        };

        for change in changes {
            apply_content_change(&mut doc.content, change);
        }

        doc.version = version;
        doc.program = None;
        true
    }

    pub fn close(&self, url: &Url) {
        self.documents.remove(url);
    }

    pub fn get_content(&self, url: &Url) -> Option<String> {
        self.documents.get(url).map(|d| d.content.clone())
    }

    pub fn set_program(&self, url: &Url, program: Program) {
        if let Some(mut doc) = self.documents.get_mut(url) {
            doc.program = Some(program);
        }
    }

    pub fn get_program(&self, url: &Url) -> Option<Program> {
        self.documents.get(url).and_then(|d| d.program.clone())
    }

    pub fn get_version(&self, url: &Url) -> Option<i32> {
        self.documents.get(url).map(|d| d.version)
    }

    pub fn first_document(&self) -> Option<(Url, String)> {
        self.documents
            .iter()
            .next()
            .map(|entry| (entry.key().clone(), entry.value().content.clone()))
    }

    pub fn all_documents(&self) -> Vec<(Url, String)> {
        self.documents
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().content.clone()))
            .collect()
    }
}

fn apply_content_change(content: &mut String, change: TextDocumentContentChangeEvent) {
    if let Some(range) = change.range {
        let start = position_to_offset(content, range.start);
        let end = position_to_offset(content, range.end);

        if start <= end && end <= content.len() {
            content.replace_range(start..end, &change.text);
            return;
        }
    }

    *content = change.text;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::analysis::parse_and_collect_diagnostics;

    #[test]
    fn cache_open_update_close_flow() {
        let cache = DocumentCache::new();
        let url = Url::parse("file:///tmp/test.brm").unwrap();

        cache.open(url.clone(), "fn main() Int { return 0 }".to_string(), 1);
        assert_eq!(
            cache.get_content(&url).as_deref(),
            Some("fn main() Int { return 0 }")
        );

        cache.update(&url, "fn main() Int { return 1 }".to_string(), 2);
        assert_eq!(
            cache.get_content(&url).as_deref(),
            Some("fn main() Int { return 1 }")
        );

        cache.close(&url);
        assert!(cache.get_content(&url).is_none());
    }

    #[test]
    fn update_clears_cached_program() {
        let cache = DocumentCache::new();
        let url = Url::parse("file:///tmp/program.brm").unwrap();
        let (program, diagnostics) =
            parse_and_collect_diagnostics("fn main() Int { return 0 }", &url);
        assert!(diagnostics.is_empty());

        cache.open(url.clone(), "fn main() Int { return 0 }".to_string(), 1);
        cache.set_program(&url, program.unwrap());
        assert!(cache.get_program(&url).is_some());

        cache.update(&url, "fn main() Int { return 2 }".to_string(), 2);
        assert!(cache.get_program(&url).is_none());
    }

    #[test]
    fn incremental_changes_update_content() {
        let cache = DocumentCache::new();
        let url = Url::parse("file:///tmp/inc.brm").unwrap();

        cache.open(url.clone(), "fn main() Int { return 0 }".to_string(), 1);
        let ok = cache.apply_changes(
            &url,
            vec![TextDocumentContentChangeEvent {
                range: Some(tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 23,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 24,
                    },
                }),
                range_length: None,
                text: "1".to_string(),
            }],
            2,
        );

        assert!(ok);
        assert_eq!(
            cache.get_content(&url).as_deref(),
            Some("fn main() Int { return 1 }")
        );
        assert_eq!(cache.get_version(&url), Some(2));
    }
}
