use super::SkTypeBase;
use shiika_core::names::ModuleFullname;
use crate::signature::MethodSignature;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkModule {
    pub base: SkTypeBase,
    pub requirements: Vec<MethodSignature>,
}

impl SkModule {
    pub fn fullname(&self) -> ModuleFullname {
        self.base.erasure.to_module_fullname()
    }
}
