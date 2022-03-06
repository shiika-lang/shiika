use crate::superclass::Superclass;
use serde::{Deserialize, Serialize};
use shiika_core::ty::*;
use std::collections::HashMap;

/// Information of a class
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub superclass: Option<Superclass>,
    pub instance_ty: TermTy,
    pub ivars: HashMap<String, super::SkIVar>,
    /// true if this class cannot be a explicit superclass.
    /// None if not applicable (eg. metaclasses cannot be a explicit superclass because there is no
    /// such syntax)
    pub is_final: Option<bool>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
}

impl ClassInfo {
    pub fn class_ty(&self) -> TermTy {
        self.instance_ty.meta_ty()
    }
}
