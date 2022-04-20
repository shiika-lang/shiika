mod sk_class;
mod sk_module;
mod wtable;
use crate::signature::MethodSignature;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::*;
pub use sk_class::SkClass;
pub use sk_module::SkModule;
use std::collections::HashMap;
pub use wtable::WTable;

pub type SkTypes = HashMap<ClassFullname, SkType>;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum SkType {
    Class(SkClass),
    Module(SkModule),
}

impl From<SkClass> for SkType {
    fn from(x: SkClass) -> Self {
        SkType::Class(x)
    }
}

impl From<SkModule> for SkType {
    fn from(x: SkModule) -> Self {
        SkType::Module(x)
    }
}

impl SkType {
    pub fn base(&self) -> &SkTypeBase {
        match self {
            SkType::Class(x) => &x.base,
            SkType::Module(x) => &x.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut SkTypeBase {
        match self {
            SkType::Class(x) => &mut x.base,
            SkType::Module(x) => &mut x.base,
        }
    }

    pub fn is_class(&self) -> bool {
        matches!(&self, SkType::Class(_))
    }

    pub fn find_method_sig(&self, name: &MethodFirstname) -> Option<&MethodSignature> {
        match self {
            SkType::Class(sk_class) => sk_class.base.method_sigs.get(name),
            SkType::Module(sk_module) => sk_module
                .requirements
                .iter()
                .find(|sig| &sig.fullname.first_name == name)
                .or_else(|| sk_module.base.method_sigs.get(name)),
        }
    }
}

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
