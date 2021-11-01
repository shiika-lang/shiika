use crate::lexer::Cursor;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{msg})")]
    ParseError {
        msg: String,
        backtrace: std::backtrace::Backtrace,
        location: Cursor,
    },
    #[error("{msg}")]
    LexError {
        msg: String,
        backtrace: std::backtrace::Backtrace,
        location: Cursor,
    },
}
