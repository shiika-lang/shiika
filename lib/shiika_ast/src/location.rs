use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone)]
pub struct Location {
    line: usize,
    col: usize,
}

impl Location {
    pub fn new(line: usize, col: usize) -> Location {
        Location { line, col }
    }
}

/// Range in a source file (end-exclusive)
#[derive(Debug, PartialEq, Clone)]
pub struct LocationSpan {
    filepath: Rc<PathBuf>,
    begin: Location,
    end: Location,
}

impl LocationSpan {
    pub fn new(filepath: &Rc<PathBuf>, begin: Location, end: Location) -> LocationSpan {
        LocationSpan {
            filepath: filepath.clone(),
            begin,
            end,
        }
    }

    pub fn todo() -> LocationSpan {
        LocationSpan {
            filepath: Rc::new(PathBuf::new()),
            begin: Location { line: 0, col: 0 },
            end: Location { line: 0, col: 0 },
        }
    }
}
