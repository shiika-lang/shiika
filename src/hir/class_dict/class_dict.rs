use crate::hir::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct ClassDict {
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_classes: HashMap<ClassFullname, SkClass>,
}
