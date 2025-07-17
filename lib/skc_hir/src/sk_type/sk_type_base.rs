use crate::method_signatures::MethodSignatures;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::{self, *};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SkTypeBase {
    pub erasure: Erasure,
    pub typarams: Vec<TyParam>,
    pub method_sigs: MethodSignatures,
    /// true if this class is an imported one
    // TODO: is this used now?
    pub foreign: bool,
}

impl SkTypeBase {
    pub fn fullname(&self) -> TypeFullname {
        self.erasure.to_type_fullname()
    }

    pub fn term_ty(&self) -> TermTy {
        let type_args = ty::typarams_to_tyargs(&self.typarams);
        self.erasure.to_term_ty().specialized_ty(type_args)
    }

    // TODO: remove this
    pub fn fullname_(&self) -> ClassFullname {
        self.erasure.to_class_fullname()
    }
}
