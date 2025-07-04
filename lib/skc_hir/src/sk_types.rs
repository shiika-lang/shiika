use crate::sk_type::{SkClass, SkType};
use crate::MethodSignature;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use std::collections::HashMap;

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

    /// Merges(copies) `other` into `self`.
    pub fn merge(&mut self, other: &SkTypes) {
        for (name, sk_type) in &other.0 {
            if let Some(existing) = self.0.get_mut(&name) {
                existing
                    .base_mut()
                    .method_sigs
                    .append(sk_type.base().method_sigs.clone());
            } else {
                self.0.insert(name.clone(), sk_type.clone());
            }
        }
    }
}
