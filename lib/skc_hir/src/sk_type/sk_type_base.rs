use crate::signatures::MethodSignatures;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::*;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SkTypeBase {
    pub erasure: Erasure,
    pub typarams: Vec<TyParam>,
    pub method_sigs: MethodSignatures,
    /// true if this class is an imported one
    pub foreign: bool,
}

impl SkTypeBase {
    pub fn fullname(&self) -> TypeFullname {
        self.erasure.to_type_fullname()
    }

    // TODO: remove this
    pub fn fullname_(&self) -> ClassFullname {
        self.erasure.to_class_fullname()
    }
}
