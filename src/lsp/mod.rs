pub mod analysis;
pub mod document;
pub mod references;
pub mod semantic_tokens;
pub mod symbols;
pub mod workspace;

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::time::{sleep, Duration};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use analysis::{
    ast_span_to_range, get_builtin_functions, get_builtin_types, get_hover_markdown, get_keywords,
    get_word_at_position, is_import_context, parse_and_collect_diagnostics,
};
use document::DocumentCache;
use references::ReferenceFinder;
use symbols::collect_document_symbols;
use workspace::{completion_packages, std_status_detail, StdSource, WorkspaceIndex};

pub struct BreomLanguageServer {
    client: Client,
    documents: Arc<DocumentCache>,
    workspace: Arc<Mutex<WorkspaceIndex>>,
}

impl BreomLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DocumentCache::new()),
            workspace: Arc::new(Mutex::new(WorkspaceIndex::new())),
        }
    }

    async fn publish_diagnostics(&self, uri: Url) {
        let content = match self.documents.get_content(&uri) {
            Some(c) => c,
            None => return,
        };

        let (program, mut diagnostics) = parse_and_collect_diagnostics(&content, &uri);

        if let Some(prog) = program {
            let extra = self.collect_import_diagnostics(&content, &prog, &uri);
            diagnostics.extend(extra);
            self.documents.set_program(&uri, prog);
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn collect_import_diagnostics(
        &self,
        content: &str,
        program: &crate::ast::program::Program,
        uri: &Url,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut workspace = match self.workspace.lock() {
            Ok(guard) => guard,
            Err(_) => return diagnostics,
        };
        let Some(snapshot) = workspace.snapshot_for_uri(uri) else {
            return diagnostics;
        };

        if snapshot.std_source == StdSource::MissingBreomHome {
            diagnostics.push(Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::WARNING),
                message:
                    "BREOM_HOME is not set. std packages are unavailable in this editor session."
                        .to_string(),
                source: Some("breom-lsp".to_string()),
                code: Some(NumberOrString::String("std-home-missing".to_string())),
                ..Default::default()
            });
        }

        for dep in &program.depends {
            let import_package = dep.path.segments.join(".");
            if snapshot.known_packages.contains(&import_package) {
                continue;
            }

            diagnostics.push(Diagnostic {
                range: ast_span_to_range(content, &dep.span),
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Unknown import package '{}'. std mode: {}",
                    import_package,
                    std_status_detail(&snapshot)
                ),
                source: Some("breom-lsp".to_string()),
                code: Some(NumberOrString::String("unknown-import".to_string())),
                ..Default::default()
            });
        }

        diagnostics
    }

    fn collect_open_document_definitions(&self, name: &str) -> Vec<Location> {
        let mut defs = Vec::new();
        for (doc_uri, content) in self.documents.all_documents() {
            let (program, _) = parse_and_collect_diagnostics(&content, &doc_uri);
            let Some(program) = program else {
                continue;
            };
            for item in &program.items {
                let span = match item {
                    crate::ast::program::TopLevelItem::Function(f) if f.name == name => {
                        Some(&f.span)
                    }
                    crate::ast::program::TopLevelItem::Struct(s) if s.name == name => Some(&s.span),
                    crate::ast::program::TopLevelItem::Interface(i) if i.name == name => {
                        Some(&i.span)
                    }
                    crate::ast::program::TopLevelItem::Enum(e) if e.name == name => Some(&e.span),
                    crate::ast::program::TopLevelItem::Define(d) if d.name == name => Some(&d.span),
                    _ => None,
                };
                if let Some(span) = span {
                    defs.push(Location {
                        uri: doc_uri.clone(),
                        range: ast_span_to_range(&content, span),
                    });
                }
            }
        }
        defs
    }

    fn collect_open_document_references(&self, name: &str) -> Vec<Location> {
        let mut refs = Vec::new();
        for (doc_uri, content) in self.documents.all_documents() {
            let (program, _) = parse_and_collect_diagnostics(&content, &doc_uri);
            let Some(program) = program else {
                continue;
            };
            let finder = ReferenceFinder::new(&content, &program);
            let mut local_refs = finder.find_references_by_name(name);
            for location in &mut local_refs {
                location.uri = doc_uri.clone();
            }
            refs.extend(local_refs);
        }
        refs
    }

    fn word_at_or_before_position(
        &self,
        content: &str,
        position: Position,
    ) -> Option<(String, Range)> {
        if let Some(found) = get_word_at_position(content, position) {
            return Some(found);
        }

        if position.character > 0 {
            let before = Position {
                line: position.line,
                character: position.character - 1,
            };
            return get_word_at_position(content, before);
        }

        None
    }

    fn collect_rename_locations(&self, uri: &Url, position: Position) -> Vec<Location> {
        let content = match self.documents.get_content(uri) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let program = match self.documents.get_program(uri) {
            Some(p) => p,
            None => return Vec::new(),
        };

        let finder = ReferenceFinder::new(&content, &program);
        let mut locations = finder.find_references(position);
        for loc in &mut locations {
            loc.uri = uri.clone();
        }

        if locations.is_empty() {
            let Some((word, _)) = self.word_at_or_before_position(&content, position) else {
                return locations;
            };

            let open_refs = self.collect_open_document_references(&word);
            let open_doc_uris = open_refs
                .iter()
                .map(|loc| loc.uri.clone())
                .collect::<HashSet<_>>();

            if let Ok(mut workspace) = self.workspace.lock() {
                let packages = workspace.definition_packages(uri, &word);
                locations = workspace.find_global_references(uri, &word, Some(&packages));
                locations.retain(|loc| !open_doc_uris.contains(&loc.uri));
                locations.extend(open_refs);
            }
        }

        locations
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BreomLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Ok(mut workspace) = self.workspace.lock() {
            workspace.configure_from_initialize(&params);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::REFACTOR,
                            CodeActionKind::new("refactor.rename"),
                        ]),
                        ..Default::default()
                    },
                )),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::get_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "breom-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Breom LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "Breom LSP server shutting down")
            .await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        self.documents.open(uri.clone(), content, version);

        let std_status = if let Ok(mut workspace) = self.workspace.lock() {
            workspace
                .snapshot_for_uri(&uri)
                .map(|snapshot| std_status_detail(&snapshot).to_string())
        } else {
            None
        };
        if let Some(message) = std_status {
            self.client.log_message(MessageType::INFO, message).await;
        }

        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;
        if !self.documents.apply_changes(&uri, changes, version) {
            return;
        }

        sleep(Duration::from_millis(120)).await;
        if self.documents.get_version(&uri) != Some(version) {
            return;
        }
        self.publish_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.close(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let content = match self.documents.get_content(uri) {
            Some(c) => c,
            None => return Ok(None),
        };

        let (word, range) = match get_word_at_position(&content, position) {
            Some(w) => w,
            None => return Ok(None),
        };

        let Some(hover_text) = get_hover_markdown(&word) else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text.to_string(),
            }),
            range: Some(range),
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let mut items = Vec::new();

        if let Some(content) = self.documents.get_content(uri) {
            if let Some(program) = self.documents.get_program(uri) {
                let finder = ReferenceFinder::new(&content, &program);

                let dot_completions = finder.collect_dot_completions(position);
                if !dot_completions.is_empty() {
                    return Ok(Some(CompletionResponse::Array(dot_completions)));
                }

                if is_import_context(&content, position) {
                    if let Ok(mut workspace) = self.workspace.lock() {
                        if let Some(snapshot) = workspace.snapshot_for_uri(uri) {
                            items.extend(completion_packages(&snapshot));
                        }
                    }
                    return Ok(Some(CompletionResponse::Array(items)));
                }

                items.extend(get_keywords());
                items.extend(get_builtin_types());
                items.extend(get_builtin_functions());
                items.extend(finder.collect_defines());
                items.extend(finder.collect_visible_symbols(position));
            }
        } else {
            items.extend(get_keywords());
            items.extend(get_builtin_types());
            items.extend(get_builtin_functions());
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let content = match self.documents.get_content(uri) {
            Some(c) => c,
            None => return Ok(None),
        };

        let program = match self.documents.get_program(uri) {
            Some(p) => p,
            None => return Ok(None),
        };

        let finder = ReferenceFinder::new(&content, &program);

        if let Some(mut location) = finder.find_definition(position) {
            location.uri = uri.clone();
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        if let Some((word, _)) = get_word_at_position(&content, position) {
            let open_defs = self.collect_open_document_definitions(&word);
            let open_doc_uris = open_defs
                .iter()
                .map(|loc| loc.uri.clone())
                .collect::<HashSet<_>>();

            if let Ok(mut workspace) = self.workspace.lock() {
                let mut defs = workspace.find_global_definitions(uri, &word);
                defs.retain(|loc| !open_doc_uris.contains(&loc.uri));
                defs.extend(open_defs);
                if defs.len() == 1 {
                    return Ok(Some(GotoDefinitionResponse::Scalar(defs[0].clone())));
                }
                if !defs.is_empty() {
                    return Ok(Some(GotoDefinitionResponse::Array(defs)));
                }
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let mut locations = self.collect_rename_locations(uri, position);

        if !include_declaration {
            let content = match self.documents.get_content(uri) {
                Some(c) => c,
                None => return Ok(Some(locations)),
            };
            if let Some((word, _)) = self.word_at_or_before_position(&content, position) {
                if let Ok(mut workspace) = self.workspace.lock() {
                    let mut defs = workspace.find_global_definitions(uri, &word);
                    defs.extend(self.collect_open_document_definitions(&word));
                    locations.retain(|loc| !defs.iter().any(|d| d == loc));
                }
            }
        }

        Ok(Some(locations))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let position = params.position;
        let content = match self.documents.get_content(uri) {
            Some(c) => c,
            None => return Ok(None),
        };

        let Some((word, range)) = self.word_at_or_before_position(&content, position) else {
            return Ok(None);
        };

        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: word,
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        if new_name.trim().is_empty() {
            return Ok(None);
        }

        let mut locations = self.collect_rename_locations(uri, position);
        if locations.is_empty() {
            return Ok(None);
        }

        let mut seen = HashSet::new();
        locations.retain(|loc| {
            seen.insert((
                loc.uri.clone(),
                loc.range.start.line,
                loc.range.start.character,
                loc.range.end.line,
                loc.range.end.character,
            ))
        });

        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for loc in locations {
            changes.entry(loc.uri).or_default().push(TextEdit {
                range: loc.range,
                new_text: new_name.clone(),
            });
        }

        for edits in changes.values_mut() {
            edits.sort_by(|a, b| {
                b.range
                    .start
                    .line
                    .cmp(&a.range.start.line)
                    .then(b.range.start.character.cmp(&a.range.start.character))
            });
        }

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let content = match self.documents.get_content(uri) {
            Some(c) => c,
            None => return Ok(None),
        };

        let range = params.range;
        let pos = range.start;
        if self.word_at_or_before_position(&content, pos).is_none() {
            return Ok(Some(Vec::new()));
        }

        let action = CodeAction {
            title: "Rename symbol".to_string(),
            kind: Some(CodeActionKind::new("refactor.rename")),
            command: Some(Command {
                title: "Rename symbol".to_string(),
                command: "editor.action.rename".to_string(),
                arguments: None,
            }),
            ..Default::default()
        };

        Ok(Some(vec![CodeActionOrCommand::CodeAction(action)]))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query;

        let mut symbols = Vec::new();
        if let Some((uri, _)) = self.documents.first_document() {
            if let Ok(mut workspace) = self.workspace.lock() {
                symbols = workspace.query_workspace_symbols(&uri, &query);
            }
        }

        Ok(Some(symbols))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        let content = match self.documents.get_content(uri) {
            Some(c) => c,
            None => return Ok(None),
        };

        let program = if let Some(program) = self.documents.get_program(uri) {
            program
        } else {
            let (program, _) = parse_and_collect_diagnostics(&content, uri);
            match program {
                Some(p) => p,
                None => return Ok(Some(DocumentSymbolResponse::Nested(Vec::new()))),
            }
        };

        let symbols = collect_document_symbols(&content, &program);
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        let content = match self.documents.get_content(uri) {
            Some(c) => c,
            None => return Ok(None),
        };

        let tokens = semantic_tokens::tokenize(&content);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }
}

pub async fn run_lsp_server() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(BreomLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
