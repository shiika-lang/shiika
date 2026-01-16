use super::class_name::*;
use super::namespace::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct ConstFullname(pub String);

impl std::fmt::Display for ConstFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ConstFullname {
    pub fn new(names: Vec<String>) -> ConstFullname {
        const_fullname(names.join("::"))
    }

    pub fn toplevel(s_: impl Into<String>) -> ConstFullname {
        toplevel_const(&s_.into())
    }
}

pub fn const_fullname(s_: impl Into<String>) -> ConstFullname {
    let s = s_.into();
    debug_assert!(!s.starts_with("::"));
    ConstFullname(format!("::{}", &s))
}

pub fn toplevel_const(first_name: &str) -> ConstFullname {
    debug_assert!(!first_name.starts_with("::"));
    ConstFullname(format!("::{}", first_name))
}

/// A const name not resolved yet
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UnresolvedConstName(pub Vec<String>);

/// Fully qualified const name.
#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedConstName {
    // REFACTOR: Just ResolvedConstName(pub Vec<String>)
    pub names: Vec<String>,
}

impl std::fmt::Display for ResolvedConstName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.string())
    }
}

impl ResolvedConstName {
    pub fn new(names: Vec<String>) -> ResolvedConstName {
        ResolvedConstName { names }
    }

    pub fn unsafe_create(s: String) -> ResolvedConstName {
        ResolvedConstName { names: vec![s] }
    }

    /// Convert to ConstFullname
    pub fn to_const_fullname(&self) -> ConstFullname {
        toplevel_const(&self.string())
    }

    /// Convert to ClassFullname
    pub fn to_class_fullname(&self) -> ClassFullname {
        class_fullname(self.string())
    }

    /// Returns string representation
    pub fn string(&self) -> String {
        self.names.join("::")
    }
}

/// Create a ResolvedConstName (which is not generic).
pub fn resolved_const_name(namespace: Namespace, names: Vec<String>) -> ResolvedConstName {
    let new_names = namespace
        .0
        .into_iter()
        .chain(names.into_iter())
        .collect::<Vec<String>>();
    ResolvedConstName { names: new_names }
}
