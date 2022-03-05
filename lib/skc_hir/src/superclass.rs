use serde::{Deserialize, Serialize};
use shiika_core::{names::*, ty, ty::*};

/// Note that superclass can have type parameters eg.
/// `class Foo<S, T> : Pair<S, Array<T>>`
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Superclass(TermTy);

impl Superclass {
    /// Create a `Superclass`
    pub fn from_ty(t: TermTy) -> Superclass {
        debug_assert!(matches!(t.body, TyBody::TyRaw { .. }));
        Superclass(t)
    }

    /// Create a (possiblly generic) `Superclass`
    pub fn new(base_name: &ModuleFullname, tyargs: Vec<TermTy>) -> Superclass {
        let t = if tyargs.is_empty() {
            ty::raw(&base_name.0)
        } else {
            ty::spe(&base_name.0, tyargs)
        };
        Superclass::from_ty(t)
    }

    /// Shortcut from a class name
    pub fn simple(s: &str) -> Superclass {
        Superclass::from_ty(ty::raw(s))
    }

    /// Default superclass (= Object)
    pub fn default() -> Superclass {
        Superclass::simple("Object")
    }

    pub fn ty(&self) -> &TermTy {
        &self.0
    }

    /// Create concrete superclass of a generic class
    pub fn substitute(&self, tyargs: &[TermTy]) -> Superclass {
        let t = self.0.substitute(tyargs, Default::default());
        Superclass::from_ty(t)
    }
}
