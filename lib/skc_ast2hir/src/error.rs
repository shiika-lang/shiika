use anyhow;
use std::backtrace::Backtrace;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{msg})")]
    SyntaxError { msg: String, backtrace: Backtrace },
    /// Errors of types
    #[error("{msg}")]
    TypeError { msg: String, backtrace: Backtrace },
    /// Invalid name
    #[error("{msg}")]
    NameError { msg: String, backtrace: Backtrace },
    /// Syntactically correct but invalid program
    #[error("{msg}")]
    ProgramError { msg: String, backtrace: Backtrace },
}

pub fn syntax_error(msg: &str) -> anyhow::Error {
    Error::SyntaxError {
        msg: msg.to_string(),
        backtrace: Backtrace::capture(),
    }
    .into()
}

pub fn type_error(msg: &str) -> anyhow::Error {
    Error::TypeError {
        msg: msg.to_string(),
        backtrace: Backtrace::capture(),
    }
    .into()
}

pub fn name_error(msg: &str) -> anyhow::Error {
    Error::NameError {
        msg: msg.to_string(),
        backtrace: Backtrace::capture(),
    }
    .into()
}

pub fn program_error(msg: &str) -> anyhow::Error {
    Error::ProgramError {
        msg: msg.to_string(),
        backtrace: Backtrace::capture(),
    }
    .into()
}