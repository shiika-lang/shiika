use crate::library::LibraryExports;
use serde::{Deserialize, Serialize};
use shiika_core::{names::*, ty::*};
use skc_hir::{SkModule, SkModules};
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VTable {
    /// List of methods, ordered by index
    fullnames: Vec<MethodFullname>,
    /// Mapping from firstname to index
    index: HashMap<MethodFirstname, usize>,
}

impl VTable {
    /// Create an empty VTable
    pub fn null() -> VTable {
        VTable {
            fullnames: vec![],
            index: HashMap::new(),
        }
    }

    /// Build a VTable of a class
    pub fn build(super_vtable: &VTable, class: &SkModule) -> VTable {
        let mut vtable = super_vtable.clone();
        for name in class.method_names() {
            if vtable.contains(&name.first_name) {
                vtable.update(name);
            } else {
                vtable.push(name);
            }
        }
        vtable
    }

    fn contains(&self, name: &MethodFirstname) -> bool {
        self.index.contains_key(name)
    }

    fn update(&mut self, name: MethodFullname) {
        let i = self.index.get(&name.first_name).unwrap();
        let elem = self.fullnames.get_mut(*i).unwrap();
        *elem = name;
    }

    fn push(&mut self, name: MethodFullname) {
        let i = self.fullnames.len();
        self.index.insert(name.first_name.clone(), i);
        self.fullnames.push(name);
    }

    /// Returns the size
    pub fn size(&self) -> usize {
        self.fullnames.len()
    }

    /// Returns the index of the method
    pub fn get(&self, name: &MethodFirstname) -> Option<&usize> {
        self.index.get(name)
    }

    /// Returns the list of method names, ordered by the index.
    pub fn to_vec(&self) -> &Vec<MethodFullname> {
        &self.fullnames
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VTables {
    // REFACTOR: how about just use `type`
    vtables: HashMap<ModuleFullname, VTable>,
}

impl VTables {
    /// Build vtables of the classes
    pub fn build(sk_modules: &SkModules, imports: &LibraryExports) -> VTables {
        let mut vtables = HashMap::new();
        let mut queue = sk_modules.keys().cloned().collect::<VecDeque<_>>();
        let null_vtable = VTable::null();
        while !queue.is_empty() {
            let name = queue.pop_front().unwrap();
            // Check if already processed
            if vtables.contains_key(&name) || imports.sk_modules.contains_key(&name) {
                continue;
            }

            let class = sk_modules
                .get(&name)
                .unwrap_or_else(|| panic!("class not found: {}", name));
            let super_vtable;
            if let Some(superclass) = &class.class_info.as_ref().unwrap().superclass {
                let super_name = superclass.ty().erasure();
                if let Some(x) = vtables.get(&super_name) {
                    super_vtable = x;
                } else if let Some(x) = imports.vtables.vtables.get(&super_name) {
                    super_vtable = x;
                } else {
                    queue.push_front(super_name);
                    queue.push_back(class.fullname.clone());
                    continue;
                }
            } else {
                // The class Object does not have a superclass.
                super_vtable = &null_vtable;
            }
            let vtable = VTable::build(super_vtable, class);
            vtables.insert(class.fullname.clone(), vtable);
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
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, ModuleFullname, VTable> {
        self.vtables.iter()
    }
}
