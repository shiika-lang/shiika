use super::SkTypeBase;
use crate::method_signature::MethodSignature;
use crate::supertype::Supertype;
use serde::{Deserialize, Serialize};
use shiika_core::names::ModuleFullname;

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
}
