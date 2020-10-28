use std::collections::HashMap;
use std::collections::VecDeque;
use crate::names::*;
use crate::ty::*;
use crate::hir::sk_class::SkClass;

#[derive(Debug, Clone)]
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
        for name in class.method_names() {
            if vtable.contains(&name.first_name) {
                vtable.update(name);
            }
            else {
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
        self.fullnames.insert(*i, name);
    }

    fn push(&mut self, name: MethodFullname) {
        let i = self.fullnames.len();
        self.index.insert(name.first_name.clone(), i);
        self.fullnames.push(name);
    }

    /// Returns the index of the method
    pub fn get(&self, name: &MethodFirstname) -> &usize {
        self.index.get(name).unwrap()
    }

    /// Returns the list of method names, ordered by the index.
    pub fn to_vec(&self) -> &Vec<MethodFullname> {
        &self.fullnames
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
            // Check if already processed
            if contents.contains_key(name) { continue }

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
    pub fn method_idx(&self, obj_ty: &TermTy, method_name: &MethodFirstname) -> &usize {
        let vtable = self.contents.get(&obj_ty.fullname).unwrap();
        vtable.get(&method_name)
    }

    // REFACTOR: it's better to implement Iterator (I just don't know how to)
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, ClassFullname, VTable> {
        self.contents.iter()
    }
}

