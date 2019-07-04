use backtrace::Backtrace;

#[derive(Debug)]
pub enum Error {
    ParseError {
        msg: String,
        location: crate::parser::lexer::Cursor,
        backtrace: Backtrace
    },
    TypeError { msg: String, backtrace: Backtrace },
    ProgramError { msg: String, backtrace: Backtrace },
    Bug { msg: String, backtrace: Backtrace },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg())
    }
}
impl std::error::Error for Error {}

impl Error {
    pub fn msg(&self) -> &str {
        match self {
            Error::ParseError { msg, .. } => msg,
            Error::TypeError { msg, .. } => msg,
            Error::ProgramError { msg, .. } => msg,
            Error::Bug { msg, .. } => msg,
        }
    }
}

pub fn type_error(msg: &str) -> Error {
    Error::TypeError {
        msg: msg.to_string(),
        backtrace: backtrace::Backtrace::new()
    }
}
