use std::collections::HashMap;

/// Witness table
#[repr(C)]
#[derive(Debug)]
pub struct WitnessTable(HashMap<u64, (usize, *const *const u8)>);

impl WitnessTable {
    pub fn new() -> WitnessTable {
        WitnessTable(HashMap::new())
    }

    /// key: Unique integer for a Shiika Module
    /// funcs: LLVM Array of function pointers
    /// len: The length of `funcs` (for safety check)
    //    pub fn insert(&mut self, key: u64, funcs: *const *const u8, len: usize) {
    //        self.0.insert(key, (len, funcs));
    //    }

    /// Get the function pointer
    /// Panics if not found
    pub fn get(&self, key: u64, idx: usize) -> *const u8 {
        let (len, funcs) = self
            .0
            .get(&key)
            .unwrap_or_else(|| panic!("[BUG] WitnessTable::get: key {} not found", key));
        if idx >= *len {
            panic!(
                "[BUG] WitnessTable::get: idx({}) is larger than len({})",
                idx, len
            );
        }
        unsafe {
            let ptr = funcs.offset(idx as isize);
            *ptr
        }
    }
}
