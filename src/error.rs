use anyhow;
use std::backtrace::Backtrace;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{msg})")]
    RunnerError { msg: String, backtrace: Backtrace,
     #[backtrace]
            source: anyhow::Error
    },
}

pub fn runner_error(msg: &str) -> anyhow::Error {
    Error::RunnerError {
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

