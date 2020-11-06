use crate::names::*;
use crate::ty::*;
use std::collections::HashMap;

/// A Shiika class, possibly generic
#[derive(Debug, PartialEq, Clone)]
pub struct SkClass {
    pub fullname: ClassFullname,
    pub typarams: Vec<TyParam>,
    pub superclass_fullname: Option<ClassFullname>,
    pub instance_ty: TermTy,
    pub ivars: HashMap<String, super::SkIVar>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
}

impl SkClass {
    pub fn class_ty(&self) -> TermTy {
        self.instance_ty.meta_ty()
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
