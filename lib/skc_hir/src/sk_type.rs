use crate::signature::MethodSignature;
use crate::superclass::Superclass;
use crate::SkIVars;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::*;
use std::collections::HashMap;

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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkClass {
    pub base: SkTypeBase,
    pub superclass: Option<Superclass>,
    pub ivars: HashMap<String, super::SkIVar>,
    /// true if this class cannot be a explicit superclass.
    /// None if not applicable (eg. metaclasses cannot be a explicit superclass because there is no
    /// such syntax)
    pub is_final: Option<bool>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkModule {
    pub base: SkTypeBase,
    pub requirements: Vec<MethodSignature>,
}

impl SkClass {
    pub fn nonmeta(base: SkTypeBase, superclass: Option<Superclass>) -> SkClass {
        SkClass {
            base,
            superclass,
            ivars: Default::default(),
            is_final: Some(false),
            const_is_obj: false,
        }
    }

    pub fn meta(base: SkTypeBase) -> SkClass {
        SkClass {
            base,
            superclass: Some(Superclass::simple("Class")),
            ivars: Default::default(),
            is_final: Some(false),
            const_is_obj: false,
        }
    }

    pub fn fullname(&self) -> ClassFullname {
        self.base.erasure.to_class_fullname()
    }

    pub fn ivars(mut self, x: SkIVars) -> Self {
        self.ivars = x;
        self
    }

    pub fn const_is_obj(mut self, x: bool) -> Self {
        self.const_is_obj = x;
        self
    }
}

impl SkTypeBase {
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
