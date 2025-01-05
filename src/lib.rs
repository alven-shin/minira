use std::collections::HashMap;

use dashmap::DashMap;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod diagnostic;
mod error;
mod file_sync;
mod format;

#[derive(Debug)]
struct Backend {
    client: Client,
    /// file path -> file contents
    opened_files: DashMap<Url, String>,
    /// list of files that have diagnostics
    diagnostics: Mutex<HashMap<Url, Vec<Diagnostic>>>,
}

impl Backend {
    fn with_client(client: Client) -> Self {
        Self {
            client,
            opened_files: DashMap::new(),
            diagnostics: Mutex::default(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                    },
                )),
                document_formatting_provider: Some(OneOf::Right(DocumentFormattingOptions {
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                })),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                concat!("hello world from ", env!("CARGO_PKG_NAME")),
            )
            .await;
        diagnostic::handle_diagnostics(self).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        file_sync::handle_did_open(self, params).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        file_sync::handle_did_close(self, params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        file_sync::handle_did_change(self, params).await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        diagnostic::handle_diagnostics(self).await;
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        format::handle_formatting(self, params).await
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::with_client);
    Server::new(stdin, stdout, socket).serve(service).await;
}
