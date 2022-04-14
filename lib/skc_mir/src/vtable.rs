use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use skc_hir::SkClass;
use std::collections::HashMap;

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
    pub fn build(super_vtable: &VTable, class: &SkClass) -> VTable {
        let mut vtable = super_vtable.clone();
        for name in class.base.method_names() {
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
