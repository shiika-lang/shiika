pub mod array;
pub mod bool;
pub mod class;
pub mod float;
pub mod int;
mod math;
pub mod object;
mod shiika_internal_memory;
pub mod shiika_internal_ptr;
pub mod string;
pub use self::array::SkAry;
pub use self::bool::SkBool;
pub use self::float::SkFloat;
pub use self::int::SkInt;
pub use self::object::SkObj;
pub use self::shiika_internal_ptr::SkPtr;
pub use self::string::SkStr;

#[no_mangle]
pub extern "C" fn shiika_lookup_wtable(receiver: SkObj, key: u64, idx: usize) -> *const u8 {
    receiver.class().witness_table().get(key, idx)
}
