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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct SkTypes(pub HashMap<TypeFullname, SkType>);

impl SkTypes {
    pub fn new(h: HashMap<TypeFullname, SkType>) -> SkTypes {
        SkTypes(h)
    }

    pub fn class_names(&self) -> impl Iterator<Item = ClassFullname> + '_ {
        self.0.values().filter_map(|sk_type| match sk_type {
            SkType::Class(x) => Some(x.fullname()),
            SkType::Module(_) => None,
        })
    }

    pub fn sk_classes(&self) -> impl Iterator<Item = &SkClass> + '_ {
        self.0.values().filter_map(|sk_type| match sk_type {
            SkType::Class(x) => Some(x),
            SkType::Module(_) => None,
        })
    }

    pub fn get_class<'hir>(&'hir self, name: &ClassFullname) -> &'hir SkClass {
        let sk_type = self.0.get(&name.to_type_fullname()).unwrap();
        if let SkType::Class(class) = sk_type {
            class
        } else {
            panic!("{} is module, not a class", name)
        }
    }
}

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

    pub fn class(&self) -> Option<&SkClass> {
        match self {
            SkType::Class(x) => Some(&x),
            SkType::Module(_) => None,
        }
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

    pub fn erasure(&self) -> &Erasure {
        &self.base().erasure
    }

    pub fn fullname(&self) -> TypeFullname {
        self.base().fullname()
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
