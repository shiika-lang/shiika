//! Provides (unsafe) utilities for pointers.
//!
//! Should be removed once `Array`, etc. is re-implemented in skc_rustlib.
use crate::builtin::object::ShiikaObject;
use crate::builtin::SkInt;
use std::convert::TryInto;
use std::os::raw::c_void;
extern "C" {
    fn box_i8ptr(p: *const u8) -> SkPtr;
    fn unbox_i8ptr(p: *const ShiikaPointer) -> *mut u8;
}

/// Instance of `::Shiika::Internal::Ptr`
#[repr(C)]
#[derive(Debug)]
pub struct SkPtr(*const ShiikaPointer);

#[repr(C)]
#[derive(Debug)]
struct ShiikaPointer {
    vtable: *const u8,
    class_obj: *const u8,
    value: *mut u8,
}

impl From<*mut c_void> for SkPtr {
    fn from(p: *mut c_void) -> Self {
        unsafe { box_i8ptr(p as *const u8) }
    }
}

impl SkPtr {
    pub fn new(p: *const u8) -> SkPtr {
        unsafe { box_i8ptr(p) }
    }

    /// Convert to Rust value
    pub fn unbox(&self) -> *const u8 {
        unsafe { unbox_i8ptr(self.0) }
    }

    /// Convert to Rust value
    pub fn unbox_mut(&self) -> *mut u8 {
        unsafe { unbox_i8ptr(self.0) }
    }
}

#[export_name = "Shiika::Internal::Ptr#+"]
pub extern "C" fn shiika_internal_ptr_add(receiver: SkPtr, n_bytes: SkInt) -> SkPtr {
    let p = receiver.unbox() as *const u8;
    let n = n_bytes.val().try_into().unwrap();
    unsafe { SkPtr::new(p.offset(n)) }
}

#[export_name = "Shiika::Internal::Ptr#load"]
pub extern "C" fn shiika_internal_ptr_load(receiver: SkPtr) -> *const ShiikaObject {
    unsafe {
        let p = receiver.unbox() as *const *const ShiikaObject;
        *p
    }
}

#[export_name = "Shiika::Internal::Ptr#store"]
pub extern "C" fn shiika_internal_ptr_store(receiver: SkPtr, object: *const ShiikaObject) {
    unsafe {
        let p = receiver.unbox_mut() as *mut *const ShiikaObject;
        *p = object
    }
}

#[export_name = "Shiika::Internal::Ptr#read"]
pub extern "C" fn shiika_internal_ptr_read(receiver: SkPtr) -> SkInt {
    unsafe {
        let b = std::ptr::read(receiver.unbox());
        (b as i64).into()
    }
}

#[export_name = "Shiika::Internal::Ptr#write"]
pub extern "C" fn shiika_internal_ptr_write(receiver: SkPtr, byte: SkInt) {
    unsafe {
        let p = receiver.unbox_mut();
        *p = byte.val().try_into().unwrap();
    }
}
