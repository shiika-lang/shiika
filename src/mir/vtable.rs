use std::collections::HashMap;
use crate::names::*;
use crate::ty::*;
use crate::hir::sk_class::SkClass;

pub type VTable = HashMap<MethodFirstname, usize>;

#[derive(Debug)]
pub struct VTables {
    pub vtables: HashMap<ClassFullname, VTable>,
}

pub fn build_vtables(classes: &HashMap<ClassFullname, SkClass>) -> VTables {
    VTables {
        vtables: HashMap::new() //TODO
    }
}

impl VTables {
    // Return the index of the method when invoking it on the object
    pub fn lookup_method_idx(&self, obj_ty: &TermTy, method_name: &MethodFirstname) -> usize {
        0
    }
}
