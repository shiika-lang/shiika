use std::collections::HashMap;
use crate::names::*;
use crate::ty::*;
use crate::hir::sk_class::SkClass;

#[derive(Debug)]
pub struct VTable {
    indices: HashMap<MethodFullname, usize>,
}

impl VTable {
    // Returns the list of methods, ordered by the index.
    pub fn to_vec(&self) -> Vec<MethodFullname> {
        let mut v = self.indices.iter().collect::<Vec<_>>();
        v.sort_unstable_by_key(|(_, i)| i.clone());
        v.into_iter().map(|(name, _)| name.clone()).collect()
    }
}

#[derive(Debug)]
pub struct VTables {
    contents: HashMap<ClassFullname, VTable>,
}

pub fn build_vtables(_classes: &HashMap<ClassFullname, SkClass>) -> VTables {
    VTables {
        contents: HashMap::new() //TODO
    }
}

impl VTables {
    // Return the index of the method when invoking it on the object
    pub fn method_idx(&self, _obj_ty: &TermTy, _method_name: &MethodFirstname) -> usize {
        0
    }

    // REFACTOR: it's better to implement Iterator (I just don't know how to)
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, ClassFullname, VTable> {
        self.contents.iter()
    }
}
