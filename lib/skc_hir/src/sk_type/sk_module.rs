use super::SkTypeBase;
use crate::method_signature::MethodSignature;
use serde::{Deserialize, Serialize};
use shiika_core::names::ModuleFullname;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SkModule {
    pub base: SkTypeBase,
    pub requirements: Vec<MethodSignature>,
}

impl SkModule {
    /// Creates new `SkModule`. Also inserts `requirements` into `method_sigs`
    pub fn new(mut base: SkTypeBase, requirements: Vec<MethodSignature>) -> SkModule {
        requirements
            .iter()
            .for_each(|sig| base.method_sigs.insert(sig.clone()));
        SkModule { base, requirements }
    }

    pub fn fullname(&self) -> ModuleFullname {
        self.base.erasure.to_module_fullname()
    }
}
