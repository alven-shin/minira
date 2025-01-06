use std::process::Stdio;

use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::error::{self, FILE_NOT_OPEN};
use crate::Backend;

pub async fn handle_formatting(
    backend: &Backend,
    params: DocumentFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let Some(document) = backend.opened_files.get(&params.text_document.uri) else {
        return Err(FILE_NOT_OPEN);
    };

    // spawn rustfmt
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn rustfmt");

    // write the contents to rustfmt's stdin
    let mut stdin = child.stdin.take().expect("failed to open stdin");
    let write_stdin = async {
        for chunk in document.value().chunks() {
            stdin
                .write_all(chunk.as_bytes())
                .await
                .expect("failed to write to stdin");
        }
        drop(stdin);
    };

    // wait for rustfmt to finish
    let ((), output) = tokio::join!(write_stdin, child.wait_with_output());
    let output = output.expect("failed to wait on rustfmt");

    if !output.status.success() {
        return Err(error::rustfmt_failed(output.status));
    }

    let new_text = String::from_utf8(output.stdout).expect("rustfmt output was not valid utf-8");
    let lines = document.len_lines() - 1;

    Ok(Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                #[expect(clippy::cast_possible_truncation)]
                line: lines as _,
                character: 0,
            },
        },
        new_text,
    }]))
}
