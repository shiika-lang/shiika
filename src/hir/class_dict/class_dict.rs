use std::collections::HashMap;
use crate::hir::*;

#[derive(Debug, PartialEq)]
pub struct ClassDict {
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_classes: HashMap<ClassFullname, SkClass>
}
