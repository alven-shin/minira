use std::borrow::Cow;
use std::process::ExitStatus;

use tower_lsp::jsonrpc::{Error, ErrorCode};

#[repr(i64)]
pub enum Code {
    FileNotOpen = 1,
    RustfmtFailed,
}

pub const FILE_NOT_OPEN: Error = Error {
    code: ErrorCode::ServerError(Code::FileNotOpen as _),
    message: Cow::Borrowed("file not open"),
    data: None,
};

pub fn rustfmt_failed(status: ExitStatus) -> Error {
    Error {
        code: ErrorCode::ServerError(Code::RustfmtFailed as _),
        message: Cow::Owned(format!("rustfmt failed with status {}", status)),
        data: None,
    }
}
