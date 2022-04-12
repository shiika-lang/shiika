use super::class_name::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct MethodFirstname(pub String);

impl std::fmt::Display for MethodFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn method_firstname(s: impl Into<String>) -> MethodFirstname {
    MethodFirstname(s.into())
}

impl MethodFirstname {
    pub fn append(&self, suffix: &str) -> MethodFirstname {
        MethodFirstname(self.0.clone() + suffix)
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct MethodFullname {
    pub full_name: String,
    pub first_name: MethodFirstname,
}

pub fn method_fullname(
    class_name: &ClassFullname,
    first_name_: impl Into<String>,
) -> MethodFullname {
    let first_name = first_name_.into();
    debug_assert!(!first_name.is_empty());
    MethodFullname {
        full_name: class_name.0.clone() + "#" + &first_name,
        first_name: MethodFirstname(first_name),
    }
}

pub fn method_fullname_raw(cls: impl Into<String>, method: impl Into<String>) -> MethodFullname {
    method_fullname(&class_fullname(cls), method)
}

impl std::fmt::Display for MethodFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.full_name)
    }
}

impl MethodFullname {
    /// Returns true if this method isn't an instance method
    pub fn is_class_method(&self) -> bool {
        self.full_name.starts_with("Meta:")
    }
}
