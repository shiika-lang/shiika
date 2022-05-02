use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use std::collections::HashMap;

/// Witness table of a Shiika class. Mapping from every Shiika module
/// which the class includes to the list of MethodFullname which are
/// the actual implementation of the methods of the module.
///
/// eg. for the class `Array` which includes `Enumerable`, WTable will
/// look like this.
///   {"Enumerable" => ["Enumerable#all?", "Array#each", ...]}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct WTable(pub HashMap<ModuleFullname, Vec<MethodFullname>>);

impl WTable {
    pub fn new(h: HashMap<ModuleFullname, Vec<MethodFullname>>) -> WTable {
        WTable(h)
    }

    // Returns empty wtable.
    pub fn default() -> WTable {
        WTable(Default::default())
    }

    pub fn get_len(&self, key: &ModuleFullname) -> usize {
        self.0.get(key).unwrap().len()
    }
}
