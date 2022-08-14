use crate::builtin::shiika_internal_ptr_typed::SkPtrTyped;
use crate::builtin::{SkAry, SkInt, SkObj};

// TODO: implement SkFn0, SkFn2..SkFn9

#[repr(C)]
pub struct SkFn1<A, R>(*const ShiikaFn1<A, R>);

#[repr(C)]
struct ShiikaFn1<A, R> {
    vtable: *const u8,
    class_obj: *const u8,
    func: SkPtrTyped<extern "C" fn(*const ShiikaFn1<A, R>, A) -> R>,
    the_self: SkObj,
    captures: SkAry<*const u8>,
    exit_status: SkInt,
}

impl<A, R> SkFn1<A, R> {
    pub fn call(&self, arg: A) -> R {
        unsafe {
            let f = (*self.0).func.get();
            f(self.0, arg)
        }
    }
}
