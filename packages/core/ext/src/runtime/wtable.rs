use shiika_ffi::core_class::{SkClass, SkObject};

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
    class.ensure_witness_table();
    class.witness_table_mut().insert(key, funcs, n_funcs);
}
