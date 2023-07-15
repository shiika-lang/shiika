//! Provides (unsafe) utilities for memories.
//!
//! Should be removed once `String` is re-implemented in skc_rustlib.
use crate::allocator;
use crate::builtin::int::SkInt;
use crate::builtin::object::SkObj;
use crate::builtin::shiika_internal_ptr::SkPtr;
use shiika_ffi_macro::shiika_method;
use std::convert::TryInto;
use std::os::raw::c_void;
use std::ptr;

#[shiika_method("Meta:Shiika::Internal::Memory#force_gc")]
pub extern "C" fn memory_force_gc() {
    bdwgc_alloc::Allocator::force_collect();
}

#[shiika_method("Meta:Shiika::Internal::Memory#memcpy")]
pub extern "C" fn memory_memcpy(_receiver: SkObj, dst: SkPtr, src: SkPtr, n_bytes: SkInt) {
    let n: usize = n_bytes.val().try_into().unwrap();
    unsafe {
        ptr::copy(src.unbox(), dst.unbox_mut(), n);
    }
}

#[shiika_method("Meta:Shiika::Internal::Memory#gc_malloc")]
pub extern "C" fn memory_gc_malloc(_receiver: SkObj, n_bytes: SkInt) -> SkPtr {
    let size = n_bytes.val() as usize;
    allocator::shiika_malloc(size).into()
}

#[shiika_method("Meta:Shiika::Internal::Memory#gc_realloc")]
pub extern "C" fn memory_gc_realloc(_receiver: SkObj, ptr: SkPtr, n_bytes: SkInt) -> SkPtr {
    let p = ptr.unbox_mut() as *mut c_void;
    let size = n_bytes.val() as usize;
    allocator::shiika_realloc(p, size).into()
}
