#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub pos: usize, // Number of bytes from the begginning of the file
}

impl Location {
    pub fn new() -> Location {
        Location {
            file: "".to_string(),
            line: 0,
            col: 0,
            pos: 0,
        }
    }
}
