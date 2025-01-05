use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::process::Command;
use tower_lsp::lsp_types::*;

use crate::Backend;

#[derive(Debug, Deserialize)]
struct DiagnosticMessage {
    manifest_path: PathBuf,
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    children: Vec<Message>,
    level: String,
    message: String,
    spans: Vec<Span>,
    code: Option<Code>,
}

#[derive(Debug, Deserialize)]
struct Span {
    line_start: u32,
    line_end: u32,
    column_start: u32,
    column_end: u32,
    file_name: PathBuf,
    label: Option<String>,
    suggested_replacement: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Code {
    code: String,
}

pub async fn handle_diagnostics(
    Backend {
        client,
        diagnostics,
        ..
    }: &Backend,
) {
    // lock mutex during the entire function
    let mut diagnostics = diagnostics.lock().await;

    // remove all existing diagnostics
    let remove_diagnostics = async {
        for (document, _) in diagnostics.drain() {
            client.publish_diagnostics(document, Vec::new(), None).await;
        }
    };

    // run clippy to get workspace diagnostics
    let output = Command::new("cargo")
        .args(["clippy", "--workspace", "--message-format", "json"])
        .output();

    // run both tasks concurrently
    let ((), output) = tokio::join!(remove_diagnostics, output);

    let output = output.expect("failed to get output from clippy");
    let output = String::from_utf8(output.stdout).expect("clippy output was not valid utf-8");
    let mut errors = Vec::new();

    // queue up all diagnostics
    for line in output.lines() {
        // not a diagnostic message
        let Ok(diagnostic): Result<DiagnosticMessage, _> = serde_json::from_str(line) else {
            continue;
        };
        dbg!();

        // recursively parse all diagnostics
        let src_root = diagnostic
            .manifest_path
            .parent()
            .expect("expected parent of Cargo.toml");
        parse_diagnostics(src_root, &mut diagnostics, &mut errors, diagnostic.message);

        // log all errors
        for error in errors.drain(..) {
            client.log_message(MessageType::ERROR, error).await;
        }
    }

    // publish all diagnostics
    for (uri, diagnostics) in diagnostics.iter() {
        client
            .publish_diagnostics(uri.clone(), diagnostics.clone(), None)
            .await;
    }
}

fn parse_diagnostics(
    src_root: &Path,
    diagnostics: &mut HashMap<Url, Vec<Diagnostic>>,
    errors: &mut Vec<String>,
    message: Message,
) {
    let severity = match message.level.as_str() {
        "error" => Some(DiagnosticSeverity::ERROR),
        "warning" => Some(DiagnosticSeverity::WARNING),
        "note" => Some(DiagnosticSeverity::INFORMATION),
        "help" => Some(DiagnosticSeverity::HINT),
        "failure-note" => Some(DiagnosticSeverity::INFORMATION),
        "error: internal compiler error" => Some(DiagnosticSeverity::ERROR),
        _ => {
            errors.push(format!("unknown severity: {}", message.level));
            None
        }
    };
    let code = message.code.map(|x| NumberOrString::String(x.code));

    for span in message.spans {
        let uri = Url::from_file_path(src_root.join(span.file_name)).expect("invalid file path");
        let range = Range {
            start: Position {
                line: span.line_start - 1,
                character: span.column_start - 1,
            },
            end: Position {
                line: span.line_end - 1,
                character: span.column_end - 1,
            },
        };

        let mut message = message.message.clone();
        if let Some(label) = span.label {
            message.push_str(&format!("\n{}", label));
        }

        diagnostics.entry(uri).or_default().push(Diagnostic {
            range,
            severity,
            code: code.clone(),
            code_description: None,
            source: Some(env!("CARGO_PKG_NAME").to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        });
    }

    for child in message.children {
        parse_diagnostics(src_root, diagnostics, errors, child);
    }
}
