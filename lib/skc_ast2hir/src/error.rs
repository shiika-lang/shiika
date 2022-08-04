#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("{msg})")]
    SyntaxError { msg: String },
    /// Errors of types
    #[error("{msg}")]
    TypeError { msg: String },
    /// Invalid name
    #[error("{msg}")]
    NameError { msg: String },
    /// Syntactically correct but invalid program
    #[error("{msg}")]
    ProgramError { msg: String },
}

pub fn syntax_error(msg: &str) -> anyhow::Error {
    Error::SyntaxError {
        msg: msg.to_string(),
    }
    .into()
}

pub fn type_error(msg: impl Into<String>) -> anyhow::Error {
    Error::TypeError { msg: msg.into() }.into()
}

pub fn name_error(msg: &str) -> anyhow::Error {
    Error::NameError {
        msg: msg.to_string(),
    }
    .into()
}

pub fn program_error(msg: &str) -> anyhow::Error {
    Error::ProgramError {
        msg: msg.to_string(),
    }
    .into()
}
