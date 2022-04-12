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

// REFACTOR: Rename to `UnresolvedTypeName` or something.
#[derive(Debug, PartialEq, Clone)]
pub struct ConstName {
    pub names: Vec<String>,
    pub args: Vec<ConstName>,
}

impl ConstName {
    /// Convert self to ResolvedConstName. `args` must be empty
    pub fn resolved(&self) -> ResolvedConstName {
        debug_assert!(self.args.is_empty());
        ResolvedConstName {
            names: self.names.clone(),
            args: vec![],
        }
    }

    /// Returns if generic
    pub fn has_type_args(&self) -> bool {
        !self.args.is_empty()
    }

    /// Make ClassFullname from self
    pub fn to_class_fullname(&self) -> ClassFullname {
        class_fullname(&self.string())
    }

    /// Return const name as String
    pub fn fullname(&self) -> String {
        "::".to_string() + &self.string()
    }

    /// Return class name as String
    fn string(&self) -> String {
        let mut s = self.names.join("::");
        if !self.args.is_empty() {
            s += "<";
            let v = self.args.iter().map(|x| x.string()).collect::<Vec<_>>();
            s += &v.join(",");
            s += ">";
        }
        s
    }
}

pub fn const_name(names: Vec<String>) -> ConstName {
    ConstName {
        names,
        args: vec![],
    }
}

/// A const name not resolved yet
#[derive(Debug, PartialEq, Clone)]
pub struct UnresolvedConstName(pub Vec<String>);

/// Fully qualified const name.
#[derive(Debug, PartialEq)]
pub struct ResolvedConstName {
    pub names: Vec<String>,
    pub args: Vec<ResolvedConstName>,
}

impl std::fmt::Display for ResolvedConstName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.string())
    }
}

impl ResolvedConstName {
    pub fn new(names: Vec<String>, args: Vec<ResolvedConstName>) -> ResolvedConstName {
        ResolvedConstName { names, args }
    }

    pub fn unsafe_create(s: String) -> ResolvedConstName {
        ResolvedConstName {
            names: vec![s],
            args: vec![],
        }
    }

    /// Returns if generic
    pub fn has_type_args(&self) -> bool {
        !self.args.is_empty()
    }

    /// Returns `self` without type arguments
    pub fn base(&self) -> ResolvedConstName {
        ResolvedConstName {
            names: self.names.clone(),
            args: Default::default(),
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
        let mut s = self.names.join("::");
        // Type args (optional)
        if !self.args.is_empty() {
            s += "<";
            let v = self.args.iter().map(|arg| arg.string()).collect::<Vec<_>>();
            s += &v.join(",");
            s += ">";
        }
        s
    }

    /// Apply type args to `self`. `self.args` must be empty.
    pub fn with_type_args(&self, args: Vec<ResolvedConstName>) -> ResolvedConstName {
        debug_assert!(self.args.is_empty());
        ResolvedConstName {
            names: self.names.clone(),
            args,
        }
    }

    /// Returns the instance type when this const refers to a class
    /// eg. "Object" -> `TermTy(Object)`
    pub fn to_ty(&self, class_typarams: &[String], method_typarams: &[String]) -> TermTy {
        if self.args.is_empty() {
            let s = self.names.join("::");
            if let Some(i) = class_typarams.iter().position(|name| *name == s) {
                ty::typaram_ref(s, ty::TyParamKind::Class, i).into_term_ty()
            } else if let Some(i) = method_typarams.iter().position(|name| *name == s) {
                ty::typaram_ref(s, ty::TyParamKind::Method, i).into_term_ty()
            } else {
                ty::raw(&self.names.join("::"))
            }
        } else {
            let type_args = self
                .args
                .iter()
                .map(|n| n.to_ty(class_typarams, method_typarams))
                .collect();
            ty::spe(&self.names.join("::"), type_args)
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
    ResolvedConstName {
        names: new_names,
        args: vec![],
    }
}

// ad hoc. Not sure I'm doing right
pub fn typaram_as_resolved_const_name(name: impl Into<String>) -> ResolvedConstName {
    resolved_const_name(Namespace::root(), vec![name.into()])
}
