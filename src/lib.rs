use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod error;
mod file_sync;
mod format;

#[derive(Debug)]
struct Backend {
    client: Client,
    opened_files: DashMap<Url, String>,
}

impl Backend {
    fn with_client(client: Client) -> Self {
        Self {
            client,
            opened_files: DashMap::new(),
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
                        save: Some(TextDocumentSyncSaveOptions::Supported(false)),
                    },
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
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
