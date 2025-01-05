use tower_lsp::lsp_types::*;

use crate::Backend;

pub fn handle_did_open(
    backend: &Backend,
    DidOpenTextDocumentParams {
        text_document: TextDocumentItem { uri, text, .. },
    }: DidOpenTextDocumentParams,
) {
    backend.opened_files.insert(uri, text);
}

pub fn handle_did_close(backend: &Backend, params: &DidCloseTextDocumentParams) {
    backend.opened_files.remove(&params.text_document.uri);
}

pub async fn handle_did_change(backend: &Backend, mut params: DidChangeTextDocumentParams) {
    let [TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        ref mut text,
    }] = &mut params.content_changes[..]
    else {
        backend
            .client
            .log_message(
                MessageType::ERROR,
                "expected single change containing entire document",
            )
            .await;
        return;
    };

    let Some(mut file) = backend.opened_files.get_mut(&params.text_document.uri) else {
        backend
            .client
            .log_message(MessageType::WARNING, "document not open")
            .await;
        return;
    };

    std::mem::swap(file.value_mut(), text);
}
