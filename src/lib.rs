#![feature(rustc_private)]

use std::collections::HashMap;

use dashmap::DashMap;
use ropey::Rope;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use lsp::diagnostic::QuickFix;
use symbol::SymbolTable;

mod lsp;
mod rustc;
mod symbol;

/// TODO: add a oncelock field to retrieve the workspace root using `cargo metadata`
#[derive(Debug)]
struct Backend {
    /// handle to the editor client to send notifications and logs
    client: Client,
    /// map of URIs to opened file contents
    opened_files: DashMap<Url, Rope>,
    /// map of URIs to list of diagnostics and quick fixes
    /// TODO: split into two maps:
    /// - files with diagnostics (makes it easy to clear diagnostics)
    /// - files with quick fixes available
    diagnostics: Mutex<HashMap<Url, Vec<(Diagnostic, QuickFix)>>>,
    /// symbols from the entire workspace
    symbols: std::sync::Mutex<SymbolTable>,
}

impl Backend {
    fn with_client(client: Client) -> Self {
        Self {
            client,
            opened_files: DashMap::new(),
            diagnostics: Mutex::default(),
            symbols: std::sync::Mutex::default(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    /// current supported capabilities:
    /// - text synchronization
    /// - formatting
    /// - diagnostics
    /// - quick fixes
    /// - hover
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
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
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
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(Vec::from([CodeActionKind::QUICKFIX])),
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                        resolve_provider: Some(false),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Options(HoverOptions {
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
        lsp::diagnostic::handle_diagnostics(self).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        lsp::file_sync::handle_did_open(self, params);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        lsp::file_sync::handle_did_close(self, &params);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        lsp::file_sync::handle_did_change(self, params).await;
    }

    /// - most of the computation happens on save
    /// - a bundled rustc compiler is called to perform type checking
    /// - TODO: use the diagnostics from the bundled compiler instead of performing a separate
    /// cargo check call
    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        lsp::diagnostic::handle_diagnostics(self).await;
        // TODO: get the manifest path using `cargo metadata`
        let results = rustc::check_workspace(
            &std::env::current_dir()
                .expect("failed to get current directory")
                .join("Cargo.toml"),
        )
        .await
        .expect("failed to check workspace");
        self.symbols
            .lock()
            .expect("poisoned")
            .merge_replace(results);
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        lsp::format::handle_formatting(self, params).await
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        lsp::code_action::handle_code_action(self, params).await
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        Ok(lsp::hover::handle_hover(self, params))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// entry point for the program
/// - if the first argument is `rustc`, run the bundled rustc compiler
/// - otherwise, run the LSP server
pub async fn run() {
    let mut args = std::env::args().skip(1).peekable(); // skip the first arg, which is the executable name

    if let Some("rustc") = args.peek().map(String::as_str) {
        rustc::compiler(&args.collect::<Vec<_>>());
    } else {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(Backend::with_client);
        Server::new(stdin, stdout, socket).serve(service).await;
    }
}
