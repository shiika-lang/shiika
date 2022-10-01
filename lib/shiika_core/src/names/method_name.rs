use super::type_name::*;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

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

#[derive(Debug, PartialEq, Clone, Eq, Serialize, Deserialize)]
pub struct MethodFullname {
    // class/module part
    pub type_name: TypeFullname,
    // method part
    pub first_name: MethodFirstname,
    // cache
    pub full_name: String,
}

impl Hash for MethodFullname {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.full_name.hash(state);
    }
}

pub fn method_fullname(type_name: TypeFullname, first_name_: impl Into<String>) -> MethodFullname {
    let first_name = first_name_.into();
    debug_assert!(!first_name.is_empty());
    debug_assert!(!first_name.starts_with('@'));
    let full_name = type_name.0.clone() + "#" + &first_name;
    MethodFullname {
        type_name,
        full_name,
        first_name: MethodFirstname(first_name),
    }
}

pub fn method_fullname_raw(cls: impl Into<String>, method: impl Into<String>) -> MethodFullname {
    method_fullname(type_fullname(cls), method)
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
