use tower_lsp::lsp_types::*;

use crate::Backend;

pub fn handle_hover(
    backend: &Backend,
    HoverParams {
        text_document_position_params:
            TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
        ..
    }: HoverParams,
) -> std::option::Option<tower_lsp::lsp_types::Hover> {
    let symbol = backend.symbols.lock().ok()?.query(&uri, position)?;

    Some(Hover {
        range: Some(symbol.range),
        contents: HoverContents::Scalar(MarkedString::from_language_code(
            "rust".to_owned(),
            format!("{}: {}", symbol.name, symbol.ty),
        )),
    })
}
