use crate::builtin::{SkClass, SkError, SkObj, SkStr};
use shiika_ffi_macro::{shiika_const_ref, shiika_method_ref};
use std::mem::ManuallyDrop;

shiika_const_ref!("::Result::Ok", SkClass, "sk_Ok");
shiika_const_ref!("::Result::Fail", SkClass, "sk_Fail");
shiika_method_ref!(
    "Meta:Result::Fail#new",
    fn(receiver: SkClass, error: SkError) -> SkFail,
    "meta_result_fail_new"
);
shiika_method_ref!(
    "Meta:Result::Ok#new",
    fn(receiver: SkClass, value: SkObj) -> SkOk,
    "meta_result_ok_new"
);

#[repr(C)]
pub union SkResult {
    pub ok: ManuallyDrop<SkOk>,
    pub fail: ManuallyDrop<SkFail>,
}
#[repr(C)]
#[derive(Debug)]
pub struct SkOk(*const ShiikaOk);
#[repr(C)]
#[derive(Debug)]
pub struct SkFail(*const ShiikaFail);

impl<A: Into<SkObj>, B: std::fmt::Display> From<Result<A, B>> for SkResult {
    fn from(x: Result<A, B>) -> Self {
        match x {
            Ok(value) => SkResult::ok(value),
            Err(e) => SkResult::fail(format!("{}", e)),
        }
    }
}

impl SkResult {
    pub fn ok(value: impl Into<SkObj>) -> SkResult {
        SkResult {
            ok: ManuallyDrop::new(SkOk::new(value)),
        }
    }

    pub fn fail(msg: impl Into<SkStr>) -> SkResult {
        SkResult {
            fail: ManuallyDrop::new(SkFail::new(msg)),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaOk {
    vtable: *const u8,
    class_obj: *const u8,
    value: SkObj,
}

impl SkOk {
    pub fn new(value: impl Into<SkObj>) -> SkOk {
        meta_result_ok_new(sk_Ok(), value.into())
    }
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaFail {
    vtable: *const u8,
    class_obj: *const u8,
}

impl SkFail {
    pub fn new(msg: impl Into<SkStr>) -> SkFail {
        meta_result_fail_new(sk_Fail(), SkError::new(msg))
    }
}
