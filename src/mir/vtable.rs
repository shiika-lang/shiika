use std::collections::HashMap;
use std::collections::VecDeque;
use crate::names::*;
use crate::ty::*;
use crate::hir::sk_class::SkClass;

#[derive(Debug)]
pub struct VTable {
    indices: HashMap<MethodFullname, usize>,
}

impl VTable {
    /// Create an empty VTable
    pub fn null() -> VTable {
        VTable { indices: HashMap::new() }
    }

    /// Build a VTable
    pub fn build(super_vtable: &VTable, class: &SkClass) -> VTable {
        let mut indices = super_vtable.indices.clone();
        let mut i = indices.len();

        let method_names = class.method_sigs.values().map(|x| x.fullname.clone());
        for name in method_names {
            indices.insert(name, i);
            i += 1;
        }
        VTable { indices }
    }

    /// Returns the list of methods, ordered by the index.
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

impl VTables {
    pub fn build(sk_classes: &HashMap<ClassFullname, SkClass>) -> VTables {
        let mut contents = HashMap::new();
        let mut queue = sk_classes.keys().collect::<VecDeque<_>>();
        let null_vtable = VTable::null();
        while !queue.is_empty() {
            let name = queue.pop_front().unwrap();
            let class = sk_classes.get(&name).unwrap();
            let super_vtable;
            if let Some(super_name) = &class.superclass_fullname {
                if let Some(x) = contents.get(super_name) {
                    super_vtable = x;
                }
                else {
                    queue.push_front(&super_name);
                    queue.push_back(&class.fullname);
                    continue;
                }
            }
            else {
                // The class Object does not have a superclass.
                super_vtable = &null_vtable;
            }
            let vtable = VTable::build(super_vtable, class);
            contents.insert(class.fullname.clone(), vtable);
        }
        VTables { contents }
    }

    // Return the index of the method when invoking it on the object
    pub fn method_idx(&self, _obj_ty: &TermTy, _method_name: &MethodFirstname) -> usize {
        0
    }

    // REFACTOR: it's better to implement Iterator (I just don't know how to)
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, ClassFullname, VTable> {
        self.contents.iter()
    }
}

