use std::collections::HashMap;
use std::rc::Rc;
use crate::ty::*;
use crate::names::*;

#[derive(Debug, PartialEq, Clone)]
pub struct SkClass {
    pub fullname: ClassFullname,
    pub superclass_fullname: Option<ClassFullname>,
    pub instance_ty: TermTy,
    pub ivars: Rc<HashMap<String, super::SkIVar>>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
}

impl SkClass {
    pub fn class_ty(&self) -> TermTy {
        self.instance_ty.meta_ty()
    }
}

