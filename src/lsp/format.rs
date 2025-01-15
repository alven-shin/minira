use std::process::Stdio;
use std::str::Lines;

use itertools::Itertools;
use similar::{DiffOp, TextDiff};
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::lsp::error::{self, FILE_NOT_OPEN};
use crate::Backend;

#[expect(clippy::too_many_lines)]
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
        let original = document.to_string();
        stdin
            .write_all(original.as_bytes())
            .await
            .expect("failed to write to stdin");
        drop(stdin); // ensure rustfmt receives EOF
        original
    };

    // wait for rustfmt to finish
    let (original, output) = tokio::join!(write_stdin, child.wait_with_output());
    let output = output.expect("failed to wait on rustfmt");

    if !output.status.success() {
        return Err(error::rustfmt_failed(output.status));
    }

    // diff the original and formatted text
    let new_text = String::from_utf8(output.stdout).expect("rustfmt output was not valid utf-8");
    let mut edits = Vec::new();
    let diff = TextDiff::from_lines(&original, &new_text);
    let mut lines = new_text.lines();

    // convert the diff to text edits
    for op in diff.ops() {
        match op {
            DiffOp::Equal { len, .. } => {
                let (_, xs) = lines.split_at(*len);
                lines = xs;
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                edits.push(TextEdit {
                    range: Range {
                        #[expect(clippy::cast_possible_truncation)]
                        start: Position {
                            line: *old_index as _,
                            character: 0,
                        },
                        #[expect(clippy::cast_possible_truncation)]
                        end: Position {
                            line: (old_index + old_len) as _,
                            character: 0,
                        },
                    },
                    new_text: String::new(),
                });
            }
            DiffOp::Insert {
                old_index, new_len, ..
            } => {
                let (new_text, xs) = lines.split_at(*new_len);
                lines = xs;

                edits.push(TextEdit {
                    range: Range {
                        #[expect(clippy::cast_possible_truncation)]
                        start: Position {
                            line: *old_index as _,
                            character: 0,
                        },
                        #[expect(clippy::cast_possible_truncation)]
                        end: Position {
                            line: *(old_index) as _,
                            character: 0,
                        },
                    },
                    new_text: new_text.into_iter().collect(),
                });
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_len,
                ..
            } => {
                let (new_text, xs) = lines.split_at(*new_len);
                lines = xs;

                edits.push(TextEdit {
                    range: Range {
                        #[expect(clippy::cast_possible_truncation)]
                        start: Position {
                            line: *old_index as _,
                            character: 0,
                        },
                        #[expect(clippy::cast_possible_truncation)]
                        end: Position {
                            line: (old_index + old_len) as _,
                            character: 0,
                        },
                    },
                    new_text: new_text.collect(),
                });
            }
        }
    }

    debug_assert_eq!(lines.next(), None);
    Ok(Some(edits))
}

trait Splitable<'a> {
    fn split_at(self, n: usize) -> (impl Iterator<Item = &'a str>, Self);
}

impl<'a> Splitable<'a> for Lines<'a> {
    fn split_at(mut self, n: usize) -> (impl Iterator<Item = &'a str>, Self) {
        let x = (0..n)
            .filter_map(|_| self.next())
            .collect::<Vec<_>>()
            .into_iter()
            .interleave_shortest(std::iter::repeat("\n"));
        (x, self)
    }
}
