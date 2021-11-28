use crate::lexer::Cursor;
use std::backtrace::Backtrace;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error on parsing
    #[error("{msg})")]
    ParseError {
        msg: String,
        backtrace: Backtrace,
        location: Cursor,
    },
    /// Error on tokenizing
    #[error("{msg}")]
    LexError {
        msg: String,
        backtrace: Backtrace,
        location: Cursor,
    },
}
