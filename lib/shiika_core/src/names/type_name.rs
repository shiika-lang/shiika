use super::class_name::{class_fullname, ClassFullname};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct TypeFullname(pub String);

impl std::fmt::Display for TypeFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TypeFullname {
    pub fn new(s: impl Into<String>, is_meta: bool) -> TypeFullname {
        if is_meta {
            metatype_fullname(s)
        } else {
            type_fullname(s)
        }
    }

    // TODO: remove this
    pub fn _to_class_fullname(&self) -> ClassFullname {
        class_fullname(&self.0)
    }
}

pub fn type_fullname(s: impl Into<String>) -> TypeFullname {
    let name = s.into();
    debug_assert!(name != "Meta:");
    debug_assert!(!name.starts_with("::"));
    debug_assert!(!name.starts_with("Meta:Meta:"));
    TypeFullname(name)
}

pub fn metatype_fullname(base_: impl Into<String>) -> TypeFullname {
    let base = base_.into();
    debug_assert!(!base.is_empty());
    if base == "Metaclass" || base.starts_with("Meta:") {
        type_fullname("Metaclass")
    } else {
        type_fullname(&("Meta:".to_string() + &base))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct UnresolvedTypeName {
    pub names: Vec<String>,
    pub args: Vec<UnresolvedTypeName>,
}

pub fn unresolved_type_name(names: Vec<String>) -> UnresolvedTypeName {
    UnresolvedTypeName {
        names,
        args: vec![],
    }
}
