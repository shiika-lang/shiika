use super::SkTypeBase;
use crate::method_signature::MethodSignature;
use crate::supertype::Supertype;
use serde::{Deserialize, Serialize};
use shiika_core::names::{method_firstname, MethodFirstname, ModuleFullname};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SkModule {
    pub base: SkTypeBase,
    pub requirements: Vec<MethodSignature>,
    /// Prerequisite modules (eg. `Equatable<X>` of `module Comparable<X> : Equatable<X>`)
    #[serde(default)]
    pub supers: Vec<Supertype>,
}

impl SkModule {
    /// Creates new `SkModule`. Also inserts `requirements` into `method_sigs`
    pub fn new(
        mut base: SkTypeBase,
        requirements: Vec<MethodSignature>,
        supers: Vec<Supertype>,
    ) -> SkModule {
        requirements
            .iter()
            .for_each(|sig| base.method_sigs.insert(sig.clone()));
        SkModule {
            base,
            requirements,
            supers,
        }
    }

    pub fn fullname(&self) -> ModuleFullname {
        self.base.erasure.to_module_fullname()
    }

    /// Returns the prerequisite modules specialized with `tyargs` (the type
    /// arguments of this module).
    /// eg. `[Equatable<Int>]` for `Comparable<Int>`
    pub fn specialized_supers(&self, tyargs: &[shiika_core::ty::TermTy]) -> Vec<Supertype> {
        if tyargs.is_empty() {
            return self.supers.clone();
        }
        self.supers.iter().map(|s| s.substitute(tyargs)).collect()
    }

    /// Name of the hidden method which returns the class object of the
    /// `idx`th type argument of this module (eg. `Int` of `Container<Int>`).
    ///
    /// A module method cannot get its type arguments from `self.class`,
    /// because the type parameters of the module are not those of the class
    /// (eg. `E` of `Enumerable<E>` is `Pair<K, V>` for `Dict<K, V>`). So every
    /// generic module requires these methods and every class which
    /// implements it gets them automatically.
    pub fn tyarg_method_name(fullname: &ModuleFullname, idx: usize) -> MethodFirstname {
        method_firstname(format!(
            "{}{}_{}",
            TYARG_METHOD_PREFIX,
            fullname.0.replace("::", "_"),
            idx
        ))
    }

    /// Returns true if `name` is made by `tyarg_method_name`.
    pub fn is_tyarg_method(name: &MethodFirstname) -> bool {
        name.0.starts_with(TYARG_METHOD_PREFIX)
    }
}

const TYARG_METHOD_PREFIX: &str = "_tyarg_";
