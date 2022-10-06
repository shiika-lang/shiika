use super::class_name::{class_fullname, metaclass_fullname, ClassFullname};
use super::const_name::{toplevel_const, ConstFullname};
use super::method_name::{method_fullname_raw, MethodFirstname, MethodFullname};
use crate::{ty, ty::TermTy};
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

    pub fn is_meta(&self) -> bool {
        self.0.starts_with("Meta:")
    }

    pub fn as_class_fullname(self) -> ClassFullname {
        class_fullname(self.0)
    }

    pub fn to_const_fullname(&self) -> ConstFullname {
        toplevel_const(&self.0)
    }

    pub fn to_ty(&self) -> TermTy {
        if self.is_meta() {
            ty::meta(&self.0.clone().split_off(5))
        } else if self.0 == "Metaclass" {
            ty::new("Metaclass", Default::default(), true)
        } else {
            ty::raw(&self.0)
        }
    }

    pub fn method_fullname(&self, method_firstname: &MethodFirstname) -> MethodFullname {
        method_fullname_raw(&self.0, &method_firstname.0)
    }

    pub fn meta_name(&self) -> ClassFullname {
        metaclass_fullname(&self.0)
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
