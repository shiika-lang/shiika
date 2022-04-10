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
