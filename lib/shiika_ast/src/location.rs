use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub line: usize,
    pub col: usize,
    pub pos: usize,
}

impl Location {
    pub fn new(line: usize, col: usize, pos: usize) -> Location {
        Location { line, col, pos }
    }
}

/// Range in a source file (end-exclusive)
#[derive(PartialEq, Eq, Clone)]
pub enum LocationSpan {
    Empty,
    Just {
        filepath: Rc<PathBuf>,
        begin: Location,
        end: Location,
    },
}

impl fmt::Debug for LocationSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocationSpan::Empty => write!(f, "LocationSpan(Empty)"),
            LocationSpan::Just {
                filepath,
                begin,
                end,
            } => write!(
                f,
                "LocationSpan(`{}`{}:{}~{}:{})",
                filepath.to_string_lossy(),
                begin.line,
                begin.col,
                end.line,
                end.col,
            ),
        }
    }
}

impl LocationSpan {
    pub fn new(filepath: &Rc<PathBuf>, begin: Location, end: Location) -> LocationSpan {
        if begin.pos > end.pos {
            println!(
                "[BUG] invalid LocationSpan pos (begin: {}, end: {})",
                begin.pos, end.pos
            );
            return LocationSpan::internal();
        }
        LocationSpan::Just {
            filepath: filepath.clone(),
            begin,
            end,
        }
    }

    pub fn merge(begin: &LocationSpan, end: &LocationSpan) -> LocationSpan {
        match (begin, end) {
            (LocationSpan::Empty, LocationSpan::Empty) => LocationSpan::Empty,
            (LocationSpan::Just { .. }, LocationSpan::Empty) => begin.clone(),
            (LocationSpan::Empty, LocationSpan::Just { .. }) => end.clone(),
            (
                LocationSpan::Just {
                    filepath, begin, ..
                },
                LocationSpan::Just {
                    filepath: filepath2,
                    end,
                    ..
                },
            ) if filepath == filepath2 => Self::new(filepath, begin.clone(), end.clone()),
            _ => {
                println!(
                    "[BUG] invalid LocationSpan (begin: {:?}, end: {:?})",
                    begin, end
                );
                LocationSpan::Empty
            }
        }
    }

    pub fn get_begin(&self) -> Location {
        match self {
            LocationSpan::Just { begin, .. } => begin.clone(),
            _ => panic!("get_end called on Empty"),
        }
    }

    pub fn get_end(&self) -> Location {
        match self {
            LocationSpan::Just { end, .. } => end.clone(),
            _ => panic!("get_end called on Empty"),
        }
    }

    /// Denotes that this ast or hir does not correspond to any source text.
    pub fn internal() -> LocationSpan {
        LocationSpan::Empty
    }

    // Used as placeholder.
    // TODO: remove this
    pub fn todo() -> LocationSpan {
        LocationSpan::Empty
    }
}
