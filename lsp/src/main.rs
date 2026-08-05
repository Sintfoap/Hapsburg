mod analysis;
mod docs_static;

use analysis::DocAnalysis;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    docs: Mutex<HashMap<Url, (String, DocAnalysis)>>,
}

impl Backend {
    async fn set_text_and_publish(&self, uri: Url, text: String) {
        let analysis = analysis::analyze(&text);
        let diags = analysis.diagnostics.clone();
        self.docs.lock().unwrap().insert(uri.clone(), (text, analysis));
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    async fn republish(&self, uri: Url) {
        let existing = self.docs.lock().unwrap().get(&uri).map(|(t, _)| t.clone());
        if let Some(text) = existing {
            self.set_text_and_publish(uri, text).await;
        }
    }

    fn with_doc<T>(&self, uri: &Url, f: impl FnOnce(&str, &DocAnalysis) -> T) -> Option<T> {
        let docs = self.docs.lock().unwrap();
        let (text, analysis) = docs.get(uri)?;
        Some(f(text, analysis))
    }
}

/// Extract the identifier (or `::`-qualified path) under a cursor position
/// by scanning the raw source line — not an AST position lookup, since the
/// AST doesn't carry column spans (see README). Good enough to drive
/// hover/definition/completion by name.
fn word_at(text: &str, pos: Position) -> Option<(String, Range)> {
    let line = text.lines().nth(pos.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let idx = (pos.character as usize).min(chars.len());
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == ':';

    let mut start = idx;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    let mut end = idx;
    while end < chars.len() && is_word(chars[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    let raw: String = chars[start..end].iter().collect();
    let trimmed = raw.trim_matches(':');
    if trimmed.is_empty() {
        return None;
    }
    Some((
        trimmed.to_string(),
        Range::new(Position::new(pos.line, start as u32), Position::new(pos.line, end as u32)),
    ))
}

fn class_hover(info: &analysis::ClassInfo) -> String {
    let mut s = format!("**dynasty {}**", info.name);
    if info.founder {
        s.push_str(" *(founder)*");
    }
    if !info.parents.is_empty() {
        s.push_str(&format!("  \ndescends `{}`", info.parents.join("`, `")));
    }
    if info.pedigree.len() > 1 {
        s.push_str(&format!("\n\n**Pedigree:** `{}`", info.pedigree.join(" -> ")));
    }
    if !info.traits.is_empty() {
        s.push_str("\n\n**Traits:**\n");
        for t in &info.traits {
            s.push_str(&format!("- `{}: {}`\n", t.name, t.ty_display));
        }
    }
    if !info.methods.is_empty() {
        s.push_str("\n**Methods:**\n");
        for m in &info.methods {
            let mut line = format!("- `{}`", m.sig_display);
            if m.is_abstract {
                line.push_str(" *(abstract)*");
            }
            if m.owner != info.name {
                line.push_str(&format!(" — inherited from `{}`", m.owner));
            }
            s.push_str(&line);
            s.push('\n');
        }
    }
    s
}

/// Which class/method "contains" a line, approximated from declaration
/// start lines only (the AST has no end positions) — the class/method
/// whose start line is the closest one at-or-before `line`.
fn enclosing_class<'a>(analysis: &'a DocAnalysis, line: u32) -> Option<&'a analysis::ClassInfo> {
    analysis
        .class_order
        .iter()
        .filter(|(l, _)| *l <= line)
        .max_by_key(|(l, _)| *l)
        .and_then(|(_, name)| analysis.classes.get(name))
}

fn hover_text(analysis: &DocAnalysis, word: &str, line: u32) -> Option<String> {
    if let Some(info) = analysis.classes.get(word) {
        return Some(class_hover(info));
    }
    if let Some(doc) = docs_static::KEYWORDS.iter().find(|(k, _)| *k == word) {
        return Some(format!("**{}** (keyword)\n\n{}", word, doc.1));
    }
    if let Some(doc) = docs_static::BUILTINS.iter().find(|(k, _)| *k == word) {
        return Some(format!("**{}**\n\n{}", word, doc.1));
    }
    if let Some((_, name, ty)) = analysis
        .var_hints
        .iter()
        .filter(|(l, n, _)| *l <= line && n == word)
        .max_by_key(|(l, _, _)| *l)
    {
        return Some(format!("```\n{}: {}\n```\n*(local binding)*", name, ty));
    }
    if let Some(class) = enclosing_class(analysis, line) {
        if let Some(t) = class.traits.iter().find(|t| t.name == word) {
            return Some(format!("`{}: {}`\n\ntrait of `{}`", t.name, t.ty_display, class.name));
        }
        if let Some(m) = class.methods.iter().find(|m| m.name == word) {
            let mut s = format!("`{}`", m.sig_display);
            if m.is_abstract {
                s.push_str(" *(abstract)*");
            }
            if m.owner != class.name {
                s.push_str(&format!("\n\ninherited from `{}`", m.owner));
            }
            return Some(s);
        }
    }
    None
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "hapsburg-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "hapsburg-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.set_text_and_publish(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.pop() {
            self.set_text_and_publish(params.text_document.uri, change.text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.republish(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.docs.lock().unwrap().remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let result = self.with_doc(&uri, |text, analysis| {
            let (word, range) = word_at(text, pos)?;
            let content = hover_text(analysis, &word, pos.line)?;
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value: content }),
                range: Some(range),
            })
        });
        Ok(result.flatten())
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let result = self.with_doc(&uri, |text, analysis| {
            let (word, _) = word_at(text, pos)?;
            let class = analysis.classes.get(&word)?;
            let range = Range::new(Position::new(class.line, 0), Position::new(class.line, u32::MAX));
            Some(GotoDefinitionResponse::Scalar(Location::new(uri.clone(), range)))
        });
        Ok(result.flatten())
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let result = self.with_doc(&uri, |text, analysis| {
            let total_lines = text.lines().count() as u32;
            let mut order = analysis.class_order.clone();
            order.sort_by_key(|(l, _)| *l);
            let mut symbols = Vec::new();
            for (i, (start, name)) in order.iter().enumerate() {
                let end = order.get(i + 1).map(|(l, _)| *l).unwrap_or(total_lines);
                let Some(info) = analysis.classes.get(name) else { continue };
                let range = Range::new(Position::new(*start, 0), Position::new(end, 0));
                let selection = Range::new(Position::new(*start, 0), Position::new(*start, u32::MAX));

                let mut children = Vec::new();
                for t in &info.traits {
                    children.push(make_symbol(
                        &t.name,
                        Some(t.ty_display.clone()),
                        SymbolKind::FIELD,
                        Range::new(Position::new(t.line, 0), Position::new(t.line, u32::MAX)),
                    ));
                }
                for m in &info.methods {
                    children.push(make_symbol(
                        &m.name,
                        Some(m.sig_display.clone()),
                        SymbolKind::METHOD,
                        Range::new(Position::new(m.line, 0), Position::new(m.line, u32::MAX)),
                    ));
                }

                #[allow(deprecated)]
                symbols.push(DocumentSymbol {
                    name: name.clone(),
                    detail: if info.founder { Some("founder".to_string()) } else { None },
                    kind: SymbolKind::CLASS,
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range: selection,
                    children: if children.is_empty() { None } else { Some(children) },
                });
            }
            DocumentSymbolResponse::Nested(symbols)
        });
        Ok(result)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let result = self.with_doc(&uri, |_text, analysis| {
            let mut items = Vec::new();
            for (name, doc) in docs_static::KEYWORDS.iter() {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("keyword".to_string()),
                    documentation: Some(Documentation::String(doc.to_string())),
                    ..Default::default()
                });
            }
            for (name, doc) in docs_static::BUILTINS.iter() {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("builtin".to_string()),
                    documentation: Some(Documentation::String(doc.to_string())),
                    ..Default::default()
                });
            }
            for name in analysis.classes.keys() {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("dynasty".to_string()),
                    ..Default::default()
                });
            }
            items
        });
        Ok(result.map(CompletionResponse::Array))
    }
}

fn make_symbol(name: &str, detail: Option<String>, kind: SymbolKind, range: Range) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: name.to_string(),
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs: Mutex::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
