use std::path::PathBuf;

pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
}

impl SourceFile {
    pub fn new(path: PathBuf, content: String) -> SourceFile {
        SourceFile { path, content }
    }
}
