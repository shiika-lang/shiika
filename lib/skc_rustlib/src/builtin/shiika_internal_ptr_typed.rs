//! Instance of `::Shiika::Internal::Ptr` but typed.
//!
//! Should be removed once `Array`, etc. is re-implemented in skc_rustlib.
use std::marker::PhantomData;

#[repr(C)]
#[derive(Debug)]
pub struct SkPtrTyped<T>(*const ShiikaPointerTyped<T>);

#[repr(C)]
#[derive(Debug)]
struct ShiikaPointerTyped<T> {
    vtable: *const u8,
    class_obj: *const u8,
    /// The wrapped pointer
    value: T,
    /// Type marker
    _marker: PhantomData<T>,
}

impl<T> SkPtrTyped<T> {
    pub fn get(&self) -> &T {
        unsafe { &(*self.0).value }
    }
}
