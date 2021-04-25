use backtrace::Backtrace;

#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub backtrace: Backtrace,
    pub details: ErrorDetails,
    pub source: Option<Box<dyn std::error::Error>>,
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
    // Invalid name
    NameError,
    // Syntactically correct but not a valid program (eg. "no such method")
    ProgramError,
    // Errors from crate::runner
    RunnerError,
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
        source: None,
    }
}

pub fn type_error(msg: &str) -> Error {
    Error {
        msg: msg.to_string(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::TypeError,
        source: None,
    }
}

pub fn name_error(msg: &str) -> Error {
    Error {
        msg: msg.to_string(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::NameError,
        source: None,
    }
}

pub fn program_error(msg: &str) -> Error {
    Error {
        msg: msg.to_string(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::ProgramError,
        source: None,
    }
}

pub fn runner_error(msg: impl Into<String>, source: Box<dyn std::error::Error>) -> Error {
    Error {
        msg: msg.into(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::RunnerError,
        source: Some(source),
    }
}

pub fn plain_runner_error(msg: impl Into<String>) -> Error {
    Error {
        msg: msg.into(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::RunnerError,
        source: None,
    }
}

pub fn bug(msg: impl Into<String>) -> Error {
    Error {
        msg: msg.into(),
        backtrace: backtrace::Backtrace::new(),
        details: ErrorDetails::Bug,
        source: None,
    }
}

pub fn must_be_some<T>(o: Option<T>, msg: String) -> T {
    o.unwrap_or_else(|| panic!("{}", msg))
}
