use super::class_name::*;
use super::namespace::*;
use crate::{ty, ty::TermTy};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct ConstFullname(pub String);

impl std::fmt::Display for ConstFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn const_fullname(s_: impl Into<String>) -> ConstFullname {
    let s = s_.into();
    debug_assert!(!s.starts_with("::"));
    ConstFullname(format!("::{}", &s))
}

pub fn toplevel_const(first_name: &str) -> ConstFullname {
    debug_assert!(!first_name.starts_with("::"));
    ConstFullname(format!("::{}", first_name))
}

/// A const name not resolved yet
#[derive(Debug, PartialEq, Clone)]
pub struct UnresolvedConstName(pub Vec<String>);

/// Fully qualified const name.
#[derive(Debug, PartialEq)]
pub struct ResolvedConstName {
    pub names: Vec<String>,
}

impl std::fmt::Display for ResolvedConstName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.string())
    }
}

impl ResolvedConstName {
    pub fn new(names: Vec<String>) -> ResolvedConstName {
        ResolvedConstName { names }
    }

    pub fn unsafe_create(s: String) -> ResolvedConstName {
        ResolvedConstName { names: vec![s] }
    }

    /// Returns `self` without type arguments
    pub fn base(&self) -> ResolvedConstName {
        ResolvedConstName {
            names: self.names.clone(),
        }
    }

    /// Convert to ConstFullname
    pub fn to_const_fullname(&self) -> ConstFullname {
        toplevel_const(&self.string())
    }

    /// Convert to ClassFullname
    pub fn to_class_fullname(&self) -> ClassFullname {
        class_fullname(self.string())
    }

    /// Returns string representation
    pub fn string(&self) -> String {
        self.names.join("::")
    }

    /// Returns the instance type when this const refers to a class
    /// eg. "Object" -> `TermTy(Object)`
    pub fn to_ty(&self, class_typarams: &[String], method_typarams: &[String]) -> TermTy {
        let s = self.names.join("::");
        if let Some(i) = class_typarams.iter().position(|name| *name == s) {
            ty::typaram_ref(s, ty::TyParamKind::Class, i).into_term_ty()
        } else if let Some(i) = method_typarams.iter().position(|name| *name == s) {
            ty::typaram_ref(s, ty::TyParamKind::Method, i).into_term_ty()
        } else {
            ty::raw(&self.names.join("::"))
        }
    }
}

/// Create a ResolvedConstName (which is not generic).
pub fn resolved_const_name(namespace: Namespace, names: Vec<String>) -> ResolvedConstName {
    let new_names = namespace
        .0
        .into_iter()
        .chain(names.into_iter())
        .collect::<Vec<String>>();
    ResolvedConstName { names: new_names }
}
