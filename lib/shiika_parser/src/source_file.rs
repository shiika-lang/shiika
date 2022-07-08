use std::path::PathBuf;
use std::rc::Rc;

pub struct SourceFile {
    pub path: Rc<PathBuf>,
    pub content: String,
}

impl SourceFile {
    pub fn new(path: PathBuf, content: String) -> SourceFile {
        SourceFile {
            path: Rc::new(path),
            content,
        }
    }
}
