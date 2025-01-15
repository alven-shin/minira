use ropey::Rope;
use tower_lsp::lsp_types::*;

use crate::Backend;

pub fn handle_did_open(
    backend: &Backend,
    DidOpenTextDocumentParams {
        text_document: TextDocumentItem { uri, text, .. },
    }: DidOpenTextDocumentParams,
) {
    backend.opened_files.insert(uri, Rope::from(text));
}

pub fn handle_did_close(backend: &Backend, params: &DidCloseTextDocumentParams) {
    backend.opened_files.remove(&params.text_document.uri);
}

pub async fn handle_did_change(
    backend: &Backend,
    DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier { uri, .. },
        content_changes,
    }: DidChangeTextDocumentParams,
) {
    let Some(mut document) = backend.opened_files.get_mut(&uri) else {
        backend
            .client
            .log_message(MessageType::WARNING, "document not open")
            .await;
        return;
    };

    for TextDocumentContentChangeEvent { range, text, .. } in content_changes {
        let Some(Range {
            start:
                Position {
                    line: line_start,
                    character: character_start,
                },
            end:
                Position {
                    line: line_end,
                    character: character_end,
                },
        }) = range
        else {
            backend
                .client
                .log_message(MessageType::ERROR, "expected incremental change range")
                .await;
            return;
        };

        let start = document.line_to_char(line_start as _) + character_start as usize;
        let end = document.line_to_char(line_end as _) + character_end as usize;
        document.remove(start..end);
        document.insert(start, &text);
    }
}
