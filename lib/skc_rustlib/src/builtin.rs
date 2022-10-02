pub mod array;
pub mod bool;
pub mod class;
pub mod float;
mod time;
//mod fn_x;
pub mod int;
mod math;
pub mod object;
mod shiika_internal_memory;
pub mod shiika_internal_ptr;
//pub mod shiika_internal_ptr_typed;
pub mod string;
mod void;
pub use self::array::SkAry;
pub use self::bool::SkBool;
pub use self::class::SkClass;
pub use self::float::SkFloat;
//pub use self::fn_x::SkFn1;
pub use self::int::SkInt;
pub use self::object::SkObj;
pub use self::shiika_internal_ptr::SkPtr;
pub use self::string::SkStr;
pub use self::void::SkVoid;

/// Get the function pointer from wtable
#[no_mangle]
pub extern "C" fn shiika_lookup_wtable(receiver: SkObj, key: u64, idx: usize) -> *const u8 {
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
    class.witness_table_mut().insert(key, funcs, n_funcs)
}
