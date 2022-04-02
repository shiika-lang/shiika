use crate::signature::MethodSignature;
use crate::superclass::Superclass;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::*;
use std::collections::HashMap;

/// A Shiika class, possibly generic
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkClass {
    pub erasure: Erasure,
    pub typarams: Vec<TyParam>,
    pub superclass: Option<Superclass>,
    pub ivars: HashMap<String, super::SkIVar>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
    /// true if this class cannot be a explicit superclass.
    /// None if not applicable (eg. metaclasses cannot be a explicit superclass because there is no
    /// such syntax)
    pub is_final: Option<bool>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
    /// true if this class is an imported one
    pub foreign: bool,
}

impl SkClass {
    pub fn fullname(&self) -> ClassFullname {
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
