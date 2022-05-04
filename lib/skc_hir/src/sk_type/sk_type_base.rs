use crate::signature::MethodSignature;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkTypeBase {
    pub erasure: Erasure,
    pub typarams: Vec<TyParam>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
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

    /// List of method names, alphabetic order
    pub fn method_names(&self) -> Vec<MethodFullname> {
        let mut v = self
            .method_sigs
            .values()
            .map(|x| x.fullname.clone())
            .collect::<Vec<_>>();
        // Sort by first name
        v.sort_unstable_by(|a, b| a.first_name.0.cmp(&b.first_name.0));
        v
    }
}
