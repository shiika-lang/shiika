use super::method_name::*;
use super::type_name::*;
use crate::{ty, ty::TermTy};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub struct ClassFirstname(pub String);

impl std::fmt::Display for ClassFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ClassFirstname {
    pub fn add_namespace(&self, namespace: &str) -> ClassFullname {
        if namespace.is_empty() {
            class_fullname(self.0.clone())
        } else {
            class_fullname(namespace.to_string() + "::" + &self.0)
        }
    }
}

pub fn class_firstname(s: impl Into<String>) -> ClassFirstname {
    ClassFirstname(s.into())
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct ClassFullname(pub String);

impl std::fmt::Display for ClassFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ClassFullname> for TypeFullname {
    fn from(x: ClassFullname) -> Self {
        type_fullname(x.0)
    }
}

pub fn class_fullname(s: impl Into<String>) -> ClassFullname {
    let name = s.into();
    debug_assert!(name != "Meta:");
    debug_assert!(!name.starts_with("::"));
    debug_assert!(!name.starts_with("Meta:Meta:"));
    ClassFullname(name)
}

pub fn metaclass_fullname(base_: impl Into<String>) -> ClassFullname {
    let base = base_.into();
    debug_assert!(!base.is_empty());
    if base == "Metaclass" || base.starts_with("Meta:") {
        class_fullname("Metaclass")
    } else {
        class_fullname(&("Meta:".to_string() + &base))
    }
}

impl ClassFullname {
    pub fn new(s: impl Into<String>, is_meta: bool) -> ClassFullname {
        if is_meta {
            metaclass_fullname(s)
        } else {
            class_fullname(s)
        }
    }

    pub fn instance_ty(&self) -> TermTy {
        if self.0 == "Metaclass" {
            ty::new("Metaclass", Default::default(), true)
        } else if self.0.starts_with("Meta:") {
            ty::meta(&self.0.clone().split_off(5))
        } else {
            ty::raw(&self.0)
        }
    }

    pub fn is_meta(&self) -> bool {
        self.0.starts_with("Meta:")
    }

    pub fn to_ty(&self) -> TermTy {
        if self.is_meta() {
            let mut name = self.0.clone();
            name.replace_range(0..=4, "");
            ty::meta(&name)
        } else {
            self.instance_ty()
        }
    }

    pub fn to_type_fullname(&self) -> TypeFullname {
        type_fullname(&self.0)
    }

    pub fn meta_name(&self) -> ClassFullname {
        metaclass_fullname(&self.0)
    }

    pub fn method_fullname(&self, method_firstname: &MethodFirstname) -> MethodFullname {
        method_fullname(self, &method_firstname.0)
    }
}
