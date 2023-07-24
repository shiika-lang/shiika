use serde::{Deserialize, Serialize};
use shiika_core::{names::*, ty::*};

/// Represents supertype i.e. ancestor class of a class or included module of
/// a class.
/// Note that superclass can have type parameters eg.
/// `class Foo<S, T> : Pair<S, Array<T>>`
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Supertype(LitTy);

impl Supertype {
    /// Create a `Supertype`
    pub fn from_ty(t: LitTy) -> Supertype {
        Supertype(t)
    }

    /// Shortcut from a class name
    pub fn simple(s: &str) -> Supertype {
        Supertype::from_ty(LitTy::raw(s))
    }

    /// Default superclass (= Object)
    pub fn default() -> Supertype {
        Supertype::simple("Object")
    }

    pub fn ty(&self) -> &LitTy {
        &self.0
    }

    pub fn to_term_ty(&self) -> TermTy {
        self.0.to_term_ty()
    }

    pub fn type_args(&self) -> &[TermTy] {
        &self.0.type_args
    }

    pub fn erasure(&self) -> Erasure {
        self.0.erasure()
    }

    pub fn base_fullname(&self) -> ClassFullname {
        self.0.erasure().to_class_fullname()
    }

    /// Create concrete superclass of a generic class
    pub fn substitute(&self, tyargs: &[TermTy]) -> Supertype {
        let t = self.0.substitute(tyargs, Default::default());
        Supertype::from_ty(t)
    }
}
