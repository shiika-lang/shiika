use shiika_ffi::core_class::class::WitnessTable;
use shiika_ffi::core_class::{SkClass, SkObject};
use std::sync::OnceLock;

/// The wtable shared by all the metaclass objects (eg. `Meta:Int`).
/// Metaclasses cannot include modules by themselves, so all of them have
/// the same wtable as `Class` (like they share the vtable of `Metaclass`).
/// Stored as `usize` because a raw pointer is not `Sync`.
static METACLASS_WTABLE: OnceLock<usize> = OnceLock::new();

pub fn metaclass_wtable() -> *mut WitnessTable {
    *METACLASS_WTABLE.get_or_init(|| Box::into_raw(Box::new(WitnessTable::new())) as usize)
        as *mut WitnessTable
}

/// Get the function pointer from wtable
#[no_mangle]
pub extern "C" fn shiika_lookup_wtable(receiver: SkObject, key: u64, idx: usize) -> *const u8 {
    receiver.class().witness_table().get(key, idx)
}

/// Insert into wtable of the class
#[no_mangle]
pub extern "C" fn shiika_insert_wtable(
    mut class: SkClass,
    key: u64,
    funcs: *const *const u8,
    n_funcs: usize,
) {
    class.witness_table_mut().insert(key, funcs, n_funcs);
}

/// Insert into the wtable shared by the metaclass objects
#[no_mangle]
pub extern "C" fn shiika_insert_metaclass_wtable(
    key: u64,
    funcs: *const *const u8,
    n_funcs: usize,
) {
    unsafe { (*metaclass_wtable()).insert(key, funcs, n_funcs) };
}
