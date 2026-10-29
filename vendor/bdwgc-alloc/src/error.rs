use core::{
    error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug)]
pub struct Error {
    description: &'static str,
}

impl Error {
    pub const fn new(description: &'static str) -> Error {
        Error { description }
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}", self.description)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.description
    }
}
