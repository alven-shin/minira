use std::collections::HashMap;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::diagnostic::{Applicability, QuickFix};
use crate::Backend;

#[allow(clippy::module_name_repetitions)]
pub async fn handle_code_action(
    Backend { diagnostics, .. }: &Backend,
    CodeActionParams {
        text_document: TextDocumentIdentifier { uri },
        range,
        ..
    }: CodeActionParams,
) -> Result<Option<CodeActionResponse>> {
    let mut actions = Vec::new();

    for (
        diagnostic @ Diagnostic {
            range: curr_range,
            message,
            ..
        },
        QuickFix {
            suggested_replacement,
            suggestion_applicability,
        },
    ) in diagnostics.lock().await.get(&uri).unwrap_or(&Vec::new())
    {
        // filter out diagnostics without suggested replacements
        // or that are not applicable to the current range
        let Some(replacement) = suggested_replacement else {
            continue;
        };
        if !check_sub_range(*curr_range, range) {
            continue;
        }

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: message.clone(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(Vec::from([diagnostic.clone()])),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    Vec::from([TextEdit {
                        range: *curr_range,
                        new_text: replacement.to_owned(),
                    }]),
                )])),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(matches!(
                *suggestion_applicability,
                Some(Applicability::MachineApplicable),
            )),
            disabled: None,
            data: None,
        }));
    }

    Ok(Some(actions))
}

/// check if range `b` is within range `a`
fn check_sub_range(a: Range, b: Range) -> bool {
    let Range {
        start:
            Position {
                line: a_line_start,
                character: a_col_start,
            },
        end: Position {
            line: a_line_end,
            character: a_col_end,
        },
    } = a;
    let Range {
        start:
            Position {
                line: b_line_start,
                character: b_col_start,
            },
        end: Position {
            line: b_line_end,
            character: b_col_end,
        },
    } = b;

    (b_line_start > a_line_start || (b_line_start == a_line_start && b_col_start >= a_col_start))
        && (b_line_end < a_line_end || (b_line_end == a_line_end && b_col_end <= a_col_end))
}
