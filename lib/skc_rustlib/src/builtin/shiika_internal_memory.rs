//! Provides (unsafe) utilities for memories.
//!
//! Should be removed once `String` is re-implemented in skc_rustlib.
use crate::allocator;
use crate::builtin::int::SkInt;
use crate::builtin::object::SkObj;
use crate::builtin::shiika_internal_ptr::SkPtr;
use std::convert::TryInto;
use std::os::raw::c_void;
use std::ptr;

#[export_name = "Meta:Shiika::Internal::Memory#memcpy"]
pub extern "C" fn memory_memcpy(_receiver: SkObj, dst: SkPtr, src: SkPtr, n_bytes: SkInt) {
    let n: usize = n_bytes.val().try_into().unwrap();
    unsafe {
        ptr::copy(src.val(), dst.val_mut(), n);
    }
}

#[export_name = "Meta:Shiika::Internal::Memory#gc_malloc"]
pub extern "C" fn memory_gc_malloc(_receiver: SkObj, n_bytes: SkInt) -> SkPtr {
    let size = n_bytes.val() as usize;
    allocator::shiika_malloc(size).into()
}

#[export_name = "Meta:Shiika::Internal::Memory#gc_realloc"]
pub extern "C" fn memory_gc_realloc(_receiver: SkObj, ptr: SkPtr, n_bytes: SkInt) -> SkPtr {
    let p = ptr.val() as *mut c_void;
    let size = n_bytes.val() as usize;
    allocator::shiika_realloc(p, size).into()
}
