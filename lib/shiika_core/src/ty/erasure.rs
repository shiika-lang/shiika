use crate::names::{
    module_fullname, toplevel_const, ClassFullname, ConstFullname, ModuleFullname, TypeFullname,
};
use crate::ty::{self, TermTy};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Erasure {
    pub base_name: String,
    /// `true` if values of this type are classes
    pub is_meta: bool,
}

impl From<Erasure> for TypeFullname {
    fn from(x: Erasure) -> Self {
        TypeFullname::new(x.base_name, x.is_meta)
    }
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

    pub fn to_module_fullname(&self) -> ModuleFullname {
        debug_assert!(!self.is_meta);
        module_fullname(&self.base_name)
    }

    pub fn to_type_fullname(&self) -> TypeFullname {
        TypeFullname::new(&self.base_name, self.is_meta)
    }

    pub fn to_const_fullname(&self) -> ConstFullname {
        debug_assert!(self.is_meta);
        toplevel_const(&self.base_name)
    }

    pub fn to_term_ty(&self) -> TermTy {
        ty::new(self.base_name.clone(), Default::default(), self.is_meta)
    }
}
