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
    let Some(mut document) = backend.opened_files.get_mut(&params.text_document.uri) else {
        return Err(FILE_NOT_OPEN);
    };

    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn rustfmt");

    let mut stdin = child.stdin.take().expect("failed to open stdin");
    let write_stdin = async {
        stdin
            .write_all(document.value().as_bytes())
            .await
            .expect("failed to write to stdin");
        drop(stdin);
    };

    let ((), output) = tokio::join!(write_stdin, child.wait_with_output());
    let output = output.expect("failed to wait on rustfmt");

    if !output.status.success() {
        return Err(error::rustfmt_failed(output.status));
    }

    let lines = document.value().lines().count();
    let new_text = String::from_utf8(output.stdout).expect("rustfmt output was not valid utf-8");

    document.value_mut().clear();
    document.value_mut().push_str(&new_text);

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
