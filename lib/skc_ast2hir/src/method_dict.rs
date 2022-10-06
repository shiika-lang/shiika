use shiika_core::names::*;
use skc_hir::*;
use std::collections::HashMap;

/// Contains all the methods
#[derive(Debug)]
pub struct MethodDict(pub SkMethods);

impl MethodDict {
    pub fn new() -> MethodDict {
        MethodDict(HashMap::new())
    }

    /// Return the vec for the method for mutation
    pub fn add_method(&mut self, typename: TypeFullname, method: SkMethod) {
        let v: &mut Vec<SkMethod> = self.0.entry(typename).or_default();
        v.push(method);
    }
}
