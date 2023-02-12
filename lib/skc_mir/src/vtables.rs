use crate::library::LibraryExports;
use crate::vtable::VTable;
use serde::{Deserialize, Serialize};
use shiika_core::{names::*, ty::*};
use skc_hir::SkTypes;
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct VTables {
    // REFACTOR: how about just use `type`
    vtables: HashMap<ClassFullname, VTable>,
}

impl VTables {
    /// Build vtables of the classes
    pub fn build(sk_types: &SkTypes, imports: &LibraryExports) -> VTables {
        let mut vtables = HashMap::new();
        let mut queue = sk_types.class_names().collect::<VecDeque<_>>();
        let null_vtable = VTable::null();
        while !queue.is_empty() {
            let name = queue.pop_front().unwrap();
            // Check if already processed
            if vtables.contains_key(&name) || imports.sk_types.0.contains_key(&name.clone().into())
            {
                continue;
            }
            let sk_class = sk_types.get_class(&name);
            let super_vtable;
            if let Some(superclass) = &sk_class.superclass {
                let super_name = superclass.base_fullname();
                if let Some(x) = vtables.get(&super_name) {
                    super_vtable = x;
                } else if let Some(x) = imports.vtables.vtables.get(&super_name) {
                    super_vtable = x;
                } else {
                    queue.push_front(super_name);
                    queue.push_back(sk_class.fullname());
                    continue;
                }
            } else {
                // The class Object does not have a superclass.
                super_vtable = &null_vtable;
            }
            let vtable = VTable::build(super_vtable, sk_class);
            vtables.insert(sk_class.fullname(), vtable);
        }
        VTables { vtables }
    }

    /// Return the index of the method when invoking it on the object
    pub fn method_idx(
        &self,
        obj_ty: &TermTy,
        method_name: &MethodFirstname,
    ) -> Option<(&usize, usize)> {
        self.vtables.get(&obj_ty.vtable_name()).map(|vtable| {
            let idx = vtable
                .get(method_name)
                .unwrap_or_else(|| panic!("[BUG] `{}' not found in {}", &method_name, &obj_ty));
            (idx, vtable.size())
        })
    }

    /// Returns iterator over each vtable
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, ClassFullname, VTable> {
        self.vtables.iter()
    }
}
