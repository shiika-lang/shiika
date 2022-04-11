use crate::names::{class_fullname, ClassFullname};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub struct TypeFirstname(pub String);

impl std::fmt::Display for TypeFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

//impl TypeFirstname {
//    pub fn add_namespace(&self, namespace: &str) -> TypeFullname {
//        if namespace.is_empty() {
//            type_fullname(self.0.clone())
//        } else {
//            type_fullname(namespace.to_string() + "::" + &self.0)
//        }
//    }
//}
//
//pub fn type_firstname(s: impl Into<String>) -> TypeFirstname {
//    TypeFirstname(s.into())
//}

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

    pub fn is_meta(&self) -> bool {
        self.0.starts_with("Meta:")
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
