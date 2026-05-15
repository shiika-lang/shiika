use crate::core_class::object::ShiikaObject;
use crate::core_class::{SkClass, SkError, SkObject, SkString, SkVoid};
use crate::SkValue;
use shiika_ffi_macro::{shiika_const_ref, shiika_method_ref};
use std::marker::PhantomData;

shiika_const_ref!("::Result::Ok", SkClass, "sk_Ok");
shiika_const_ref!("::Result::Fail", SkClass, "sk_Fail");
shiika_method_ref!(
    "Meta:Result::Ok#new",
    fn(receiver: SkClass, value: SkObject) -> SkObject,
    "meta_result_ok_new"
);
shiika_method_ref!(
    "Meta:Result::Fail#new",
    fn(receiver: SkClass, err: SkError) -> SkObject,
    "meta_result_fail_new"
);

#[repr(C)]
#[derive(Debug)]
pub struct SkResult<T>(*const u8, PhantomData<T>);

unsafe impl<T> Send for SkResult<T> {}

impl<T> crate::SkValue for SkResult<T> {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

impl<T: SkValue> SkResult<T> {
    pub fn ok(value: T) -> SkResult<T> {
        let value_obj = SkObject(value.as_raw_u64() as *const ShiikaObject);
        let r = meta_result_ok_new(sk_Ok(), value_obj);
        SkResult(r.0 as *const u8, PhantomData)
    }

    pub fn fail(msg: impl Into<SkString>) -> SkResult<T> {
        let err = SkError::new(msg);
        let r = meta_result_fail_new(sk_Fail(), err);
        SkResult(r.0 as *const u8, PhantomData)
    }
}

impl<T: SkValue, E: std::fmt::Display> From<Result<T, E>> for SkResult<T> {
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
            Ok(_) => SkResult::ok(SkVoid::from(())),
            Err(e) => SkResult::fail(format!("{}", e)),
        }
    }
}
