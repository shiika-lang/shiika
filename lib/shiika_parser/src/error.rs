use crate::lexer::Cursor;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error on parsing
    #[error("{0})")]
    ParseError(String),
    /// Error on tokenizing
    #[error("{msg}")]
    LexError { msg: String, location: Cursor },
}
