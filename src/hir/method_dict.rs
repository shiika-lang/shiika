use std::collections::HashMap;
use crate::hir::*;
use crate::names::*;

/// Contains all the methods
#[derive(Debug)]
pub struct MethodDict {
    pub sk_methods: HashMap<ClassFullname, Vec<SkMethod>>
}

impl MethodDict {
    pub fn new() -> MethodDict {
        MethodDict {
            sk_methods: HashMap::new()
        }
    }

    /// Return the vec for the method for mutation
    pub fn add_method(&mut self,
                      classname: &ClassFullname,
                      method: SkMethod) {
        self.register_class(classname);
        let vec = self.sk_methods.get_mut(classname).unwrap();
        vec.push(method);
    }

    /// Add entry for the class if not exist.
    fn register_class(&mut self, fullname: &ClassFullname) {
        if !self.sk_methods.contains_key(fullname) {
            self.sk_methods.insert(fullname.clone(), vec![]);
        }
    }
}
