use crate::names::*;
use crate::ty;
use crate::ty::*;
use serde::{Deserialize, Serialize};

/// Note that superclass can have type parameters eg.
/// `class Foo<S, T> : Pair<S, Array<T>>`
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Superclass(TermTy);

impl Superclass {
    /// Create a `Superclass`
    pub fn new(t: TermTy) -> Superclass {
        debug_assert!(matches!(t.body, TyBody::TyRaw | TyBody::TySpe { .. }));
        Superclass(t)
    }

    /// Shortcut from a class name
    pub fn simple(s: &str) -> Superclass {
        Superclass(ty::raw(s))
    }

    /// Default superclass (= Object)
    pub fn default() -> Superclass {
        Superclass::simple("Object")
    }

    pub fn from_const_name(name: &ConstName, typarams: &[String]) -> Superclass {
        Superclass::new(name.to_ty(typarams))
    }

    pub fn ty(&self) -> &TermTy {
        &self.0
    }

    /// Create concrete superclass of a generic class
    pub fn substitute(&self, tyargs: &[TermTy]) -> Superclass {
        let t = self.0.substitute(Some(tyargs), None);
        Superclass::new(t)
    }
}
