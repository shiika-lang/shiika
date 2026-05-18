use crate::core_class::{SkClass, SkString};
use shiika_ffi_macro::{shiika_const_ref, shiika_method_ref};

shiika_const_ref!("::Error", SkClass, "sk_Error");
shiika_method_ref!(
    "Meta:Error#new",
    fn(receiver: SkClass, msg: SkString) -> SkError,
    "meta_error_new"
);

#[repr(C)]
#[derive(Debug)]
pub struct SkError(pub *const ShiikaError);

unsafe impl Send for SkError {}

impl crate::SkValue for SkError {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaError {
    vtable: *const u8,
    class_obj: *const u8,
    msg: SkString,
}

impl SkError {
    pub fn new(msg: impl Into<SkString>) -> SkError {
        meta_error_new(sk_Error(), msg.into())
    }
}
