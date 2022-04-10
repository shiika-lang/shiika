use crate::names::ClassFullname;
use crate::ty::{self, TermTy};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Erasure {
    base_name: String,
    /// `true` if values of this type are classes
    is_meta: bool,
}

impl Erasure {
    pub fn nonmeta(base_name_: impl Into<String>) -> Erasure {
        Self::new(base_name_.into(), false)
    }

    pub fn meta(base_name_: impl Into<String>) -> Erasure {
        Self::new(base_name_.into(), true)
    }

    pub fn new(base_name: String, is_meta_: bool) -> Erasure {
        let is_meta = if base_name == "Metaclass" {
            // There is no `Meta:Metaclass`
            true
        } else {
            is_meta_
        };
        Erasure { base_name, is_meta }
    }

    pub fn to_class_fullname(&self) -> ClassFullname {
        ClassFullname::new(&self.base_name, self.is_meta)
    }

    pub fn to_term_ty(&self) -> TermTy {
        ty::new(self.base_name.clone(), Default::default(), self.is_meta)
    }
}
