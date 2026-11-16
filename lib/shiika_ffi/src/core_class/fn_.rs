use crate::core_class::SkClass;

/// A Shiika lambda/block object (instance of `Fn0`, `Fn1`, ...).
#[repr(C)]
#[derive(Debug)]
pub struct SkFn(*const ShiikaFn);

unsafe impl Send for SkFn {}

impl crate::SkValue for SkFn {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

/// Layout of `Fn` objects.
/// Must match `packages/core/lib/fn.sk` (and `FN_IVAR_*` in
/// `skc_main::mirgen::lambda`).
#[repr(C)]
#[derive(Debug)]
struct ShiikaFn {
    vtable: *const u8,
    class_obj: SkClass,
    func: *const u8,
    captures: *const u8,
    exit_status: i64,
}

impl SkFn {
    /// Returns the pointer to the lambda function (`@func`).
    ///
    /// Its (lowered) signature is `(env, fn_obj, args..., cont) -> ContFuture`
    /// because lambdas are always compiled as async functions.
    pub fn func_ptr(&self) -> *const u8 {
        unsafe { (*self.0).func }
    }
}
