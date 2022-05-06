use super::signature::MethodSignature;
use serde::{Deserialize, Serialize};
use shiika_core::names::MethodFirstname;
use std::collections::HashMap;

/// A method list like an ordered map.
#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
pub struct MethodSignatures(HashMap<MethodFirstname, (MethodSignature, usize)>);

impl MethodSignatures {
    pub fn new() -> MethodSignatures {
        Default::default()
    }

    pub fn from_iterator(iter: impl Iterator<Item=MethodSignature>) -> MethodSignatures {
        let mut ss = MethodSignatures::new();
        iter.for_each(|s| ss.insert(s));
        ss
    }

    /// Returns the signature, if any
    pub fn get(&self, name: &MethodFirstname) -> Option<&(MethodSignature, usize)> {
        self.0.get(name)
    }

    /// Returns if the name is contained
    pub fn contains_key(&self, name: &MethodFirstname) -> bool {
        self.0.contains_key(name)
    }

    /// Insert a signature as the "last" element.
    pub fn insert(&mut self, sig: MethodSignature) {
        let n = self.0.len();
        let key = sig.fullname.first_name.clone();
        self.0.insert(key, (sig, n));
    }

    /// Destructively append `other` to `self`.
    pub fn append(&mut self, other: MethodSignatures) {
        other.into_ordered().into_iter().for_each(|(s, _)| self.insert(s));
    }

    /// Returns list of signatures in the order.
    fn into_ordered(self) -> Vec<(MethodSignature, usize)> {
        let mut v = self.0.into_values().collect::<Vec<_>>();
        // This is stable because n is unique.
        v.sort_unstable_by_key(|(_, n)| *n);
        v
    }

    /// Returns list of signatures in the order.
    pub fn to_ordered(&self) -> Vec<&(MethodSignature, usize)> {
        let mut v = self.0.values().collect::<Vec<_>>();
        // This is stable because n is unique.
        v.sort_unstable_by_key(|(_, n)| n);
        v
    }

    /// Returns iterator over signatures (not ordered.)
    pub fn unordered_iter(&self) -> impl Iterator<Item=&(MethodSignature, usize)> {
        self.0.values()
    }
}
