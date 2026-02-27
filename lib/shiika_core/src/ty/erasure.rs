use crate::names::{
    module_fullname, toplevel_const, ClassFullname, ConstFullname, ModuleFullname, Namespace,
    TypeFullname,
};
use crate::ty::{LitTy, TermTy};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Erasure {
    pub base_name: String,
    /// `true` if values of this type are classes
    pub is_meta: bool,
}

impl std::fmt::Display for Erasure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let meta = if self.base_name == "Metaclass" {
            // There is no `Meta:Metaclass`
            ""
        } else if self.is_meta {
            "Meta:"
        } else {
            ""
        };
        write!(f, "{}{}", meta, self.base_name)
    }
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

    pub fn the_metaclass() -> Erasure {
        Self::new("Metaclass".to_string(), true)
    }

    pub fn is_the_metaclass(&self) -> bool {
        self.base_name == "Metaclass"
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

    pub fn namespace(&self) -> Namespace {
        // The namespace is the base name without the last part
        // REFACTOR: self.base_name should be `Vec<String>`
        let parts: Vec<String> = self.base_name.split("::").map(String::from).collect();
        if parts.len() > 1 {
            Namespace::new(parts[..parts.len() - 1].to_vec())
        } else {
            Namespace::root()
        }
    }

    pub fn meta_erasure(&self) -> Erasure {
        if self.is_meta {
            Erasure::the_metaclass()
        } else {
            Erasure::meta(self.base_name.clone())
        }
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
        debug_assert!(
            self.is_meta || self.is_the_metaclass(),
            "{:?} is not a meta type",
            self
        );
        toplevel_const(&self.base_name)
    }

    pub fn to_term_ty(&self) -> TermTy {
        self.to_lit_ty().to_term_ty()
    }

    pub fn to_lit_ty(&self) -> LitTy {
        LitTy::new(self.base_name.clone(), Default::default(), self.is_meta)
    }
}
