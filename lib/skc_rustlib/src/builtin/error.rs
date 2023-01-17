use crate::builtin::{SkClass, SkStr};
use shiika_ffi_macro::{shiika_const_ref, shiika_method_ref};

shiika_const_ref!("::Error", SkClass, "sk_Error");
shiika_method_ref!(
    "Meta:Error#new",
    fn(receiver: SkClass, msg: SkStr) -> SkError,
    "meta_error_new"
);

#[repr(C)]
pub struct SkError(*const ShiikaError);

#[repr(C)]
#[derive(Debug)]
struct ShiikaError {
    vtable: *const u8,
    class_obj: *const u8,
    msg: SkStr,
}

impl SkError {
    pub fn new(msg: impl Into<SkStr>) -> SkError {
        meta_error_new(sk_Error(), msg.into())
    }
}
