use super::SkTypeBase;
use crate::superclass::Superclass;
use crate::{SkIVar, SkIVars};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkClass {
    pub base: SkTypeBase,
    pub superclass: Option<Superclass>,
    pub ivars: HashMap<String, SkIVar>,
    /// true if this class cannot be a explicit superclass.
    /// None if not applicable (eg. metaclasses cannot be a explicit superclass because there is no
    /// such syntax)
    pub is_final: Option<bool>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
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

    pub fn ivars(mut self, x: SkIVars) -> Self {
        self.ivars = x;
        self
    }

    pub fn const_is_obj(mut self, x: bool) -> Self {
        self.const_is_obj = x;
        self
    }
}
