use crate::class_info::ClassInfo;
use shiika_core::names::*;
use shiika_core::ty::*;
use std::collections::HashMap;
use crate::signature::MethodSignature;
use serde::{Deserialize, Serialize};

/// A Shiika module, possibly generic
/// Note that a class is a module in Shiika (as in Ruby)
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkModule {
    pub fullname: ModuleFullname,
    pub typarams: Vec<TyParam>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
    /// true if this module is an imported one
    pub foreign: bool,
    pub class_info: Option<ClassInfo>,
}

impl SkModule {
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
