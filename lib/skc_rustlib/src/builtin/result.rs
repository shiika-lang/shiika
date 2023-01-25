use crate::builtin::{SkClass, SkError, SkObj, SkStr, SkVoid};
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
    fn(receiver: SkClass, value: SkObj) -> SkOk<SkObj>,
    "meta_result_ok_new"
);

#[repr(C)]
pub union SkResult<T> {
    pub ok: ManuallyDrop<SkOk<T>>,
    pub fail: ManuallyDrop<SkFail>,
}
#[repr(C)]
#[derive(Debug)]
pub struct SkOk<T>(*const ShiikaOk<T>);
#[repr(C)]
#[derive(Debug)]
pub struct SkFail(*const ShiikaFail);

impl<T: Into<SkObj>, E: std::fmt::Display> From<Result<T, E>> for SkResult<T> {
    fn from(x: Result<T, E>) -> Self {
        match x {
            Ok(value) => SkResult::ok(value),
            Err(e) => SkResult::fail(format!("{}", e)),
        }
    }
}

impl<E: std::fmt::Display> From<Result<(), E>> for SkResult<SkVoid> {
    fn from(x: Result<(), E>) -> Self {
        match x {
            Ok(_) => SkResult::ok(().into()),
            Err(e) => SkResult::fail(format!("{}", e)),
        }
    }
}

impl<T: Into<SkObj>> SkResult<T> {
    pub fn ok(value: T) -> SkResult<T> {
        SkResult {
            ok: ManuallyDrop::new(SkOk::new(value)),
        }
    }

    pub fn fail(msg: impl Into<SkStr>) -> SkResult<T> {
        SkResult {
            fail: ManuallyDrop::new(SkFail::new(msg)),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaOk<T> {
    vtable: *const u8,
    class_obj: *const u8,
    value: T,
}

impl<T: Into<SkObj>> SkOk<T> {
    pub fn new(value: T) -> SkOk<T> {
        let ok_obj = meta_result_ok_new(sk_Ok(), value.into());
        SkOk(ok_obj.0 as *const ShiikaOk<T>)
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
