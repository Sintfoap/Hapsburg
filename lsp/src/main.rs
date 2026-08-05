use ferdinand::codegen::Codegen;
use ferdinand::parser;
use ferdinand::resolve::Resolver;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    docs: Mutex<HashMap<Url, String>>,
}

impl Backend {
    async fn publish_diagnostics(&self, uri: Url) {
        let text = match self.docs.lock().unwrap().get(&uri) {
            Some(t) => t.clone(),
            None => return,
        };
        let diagnostics = analyze(&text);
        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

/// ferdinand's own errors are plain strings, most (but not all — see
/// README) carrying a "line N" marker. Best-effort extraction rather than
/// real span tracking through the whole compiler; falls back to the top
/// of the file when a message doesn't name a line.
fn line_from_message(msg: &str) -> u32 {
    if let Some(idx) = msg.find("line ") {
        let rest = &msg[idx + 5..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<u32>() {
            if n > 0 {
                return n - 1; // LSP positions are 0-indexed
            }
        }
    }
    0
}

fn diagnostic(msg: &str, severity: DiagnosticSeverity) -> Diagnostic {
    let line = line_from_message(msg);
    let range = Range::new(Position::new(line, 0), Position::new(line, u32::MAX));
    Diagnostic {
        range,
        severity: Some(severity),
        source: Some("ferdinand".to_string()),
        message: msg.to_string(),
        ..Diagnostic::default()
    }
}

/// Run the real compiler pipeline (parse -> resolve -> codegen) purely
/// in-memory, no C emitted and no `cc` invoked, and turn whatever it
/// reports into diagnostics. Stops at the first hard error, same as
/// ferdinand itself does on the command line.
fn analyze(text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    let program = match parser::parse(text) {
        Ok(p) => p,
        Err(e) => {
            out.push(diagnostic(&e, DiagnosticSeverity::ERROR));
            return out;
        }
    };

    let mut resolver = match Resolver::new(&program) {
        Ok(r) => r,
        Err(e) => {
            out.push(diagnostic(&e.0, DiagnosticSeverity::ERROR));
            return out;
        }
    };

    if let Err(e) = resolver.check_all() {
        out.push(diagnostic(&e.0, DiagnosticSeverity::ERROR));
        return out;
    }

    for w in resolver.warnings() {
        out.push(diagnostic(w, DiagnosticSeverity::WARNING));
    }

    let mut cg = Codegen::new(&resolver);
    let birthed = cg.discover_birthed(&program);
    for class in &birthed {
        if let Err(e) = cg.gen_class(class) {
            out.push(diagnostic(&e, DiagnosticSeverity::ERROR));
            return out;
        }
    }
    if let Err(e) = cg.gen_main(&program) {
        out.push(diagnostic(&e, DiagnosticSeverity::ERROR));
    }

    out
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
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
        let uri = params.text_document.uri.clone();
        self.docs.lock().unwrap().insert(uri.clone(), params.text_document.text);
        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.pop() {
            self.docs.lock().unwrap().insert(uri.clone(), change.text);
        }
        self.publish_diagnostics(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.publish_diagnostics(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.docs.lock().unwrap().remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
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
