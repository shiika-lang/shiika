use backtrace::Backtrace;

#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub backtrace: Backtrace,
    pub details: ErrorDetails,
}
#[derive(Debug)]
pub enum ErrorDetails {
    // Error on parsing
    ParseError {
        location: crate::parser::lexer::Cursor,
    },
    // Parsing is succeeded but syntactically wrong
    SyntaxError,
    // Errors related to types
    TypeError,
    // Syntactically correct but not a valid program (eg. "no such method")
    ProgramError,
    // Not an user-error
    Bug,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for Error {}

pub fn syntax_error(msg: &str) -> Error {
    Error {
        msg: msg.to_string(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::SyntaxError,
    }
}

pub fn type_error(msg: &str) -> Error {
    Error {
        msg: msg.to_string(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::TypeError,
    }
}

pub fn program_error(msg: &str) -> Error {
    Error {
        msg: msg.to_string(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::ProgramError,
    }
}
