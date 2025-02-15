mod sk_class;
mod sk_module;
mod sk_type_base;
mod wtable;
use crate::MethodSignature;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::{self, *};
pub use sk_class::SkClass;
pub use sk_module::SkModule;
pub use sk_type_base::SkTypeBase;
use std::collections::HashMap;
pub use wtable::WTable;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
pub struct SkTypes(pub HashMap<TypeFullname, SkType>);

impl SkTypes {
    pub fn new(h: HashMap<TypeFullname, SkType>) -> SkTypes {
        SkTypes(h)
    }

    pub fn from_iterator(iter: impl Iterator<Item = SkType>) -> SkTypes {
        let mut tt = HashMap::new();
        iter.for_each(|t| {
            tt.insert(t.fullname(), t);
        });
        SkTypes(tt)
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
        let sk_type = self
            .0
            .get(&name.to_type_fullname())
            .unwrap_or_else(|| panic!("[BUG] class {} not found", name));
        if let SkType::Class(class) = sk_type {
            class
        } else {
            panic!("{} is module, not a class", name)
        }
    }

    pub fn define_method(&mut self, type_name: &TypeFullname, method_sig: MethodSignature) {
        let sk_type = self
            .0
            .get_mut(type_name)
            .unwrap_or_else(|| panic!("type '{}' not found", type_name));
        sk_type.base_mut().method_sigs.insert(method_sig);
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
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

    pub fn is_module(&self) -> bool {
        matches!(&self, SkType::Module(_))
    }

    pub fn class(&self) -> Option<&SkClass> {
        match self {
            SkType::Class(x) => Some(x),
            SkType::Module(_) => None,
        }
    }

    pub fn erasure(&self) -> &Erasure {
        &self.base().erasure
    }

    pub fn fullname(&self) -> TypeFullname {
        self.base().fullname()
    }

    // eg. TermTy(Array<T>), TermTy(Dict<K, V>)
    pub fn term_ty(&self) -> TermTy {
        let type_args = ty::typarams_to_tyargs(&self.base().typarams);
        ty::spe(self.fullname().0, type_args)
    }

    pub fn const_is_obj(&self) -> bool {
        match self {
            SkType::Class(x) => x.const_is_obj,
            SkType::Module(_) => false,
        }
    }
}
