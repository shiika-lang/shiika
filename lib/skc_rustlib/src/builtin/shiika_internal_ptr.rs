use crate::builtin::object::ShiikaObject;
use crate::builtin::{SkInt, SkObj};
use std::convert::TryInto;
use std::os::raw::c_void;
extern "C" {
    fn box_i8ptr(p: *const u8) -> SkPtr;
}

/// Instance of `::Shiika::Internal::Ptr`
#[repr(C)]
#[derive(Debug)]
pub struct SkPtr(*const ShiikaPtr);

#[repr(C)]
#[derive(Debug)]
struct ShiikaPtr {
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
    pub fn val(&self) -> *const u8 {
        unsafe { (*self.0).value }
    }

    /// Convert to Rust value
    pub fn val_mut(&self) -> *mut u8 {
        unsafe { (*self.0).value }
    }
}

#[export_name = "Shiika::Internal::Ptr#+"]
pub extern "C" fn shiika_internal_ptr_add(receiver: SkPtr, n_bytes: SkInt) -> SkPtr {
    let p = receiver.val() as *const ShiikaObject;
    let n = n_bytes.val().try_into().unwrap();
    unsafe { SkPtr::new(p.offset(n) as *const u8) }
}

#[export_name = "Shiika::Internal::Ptr#load"]
pub extern "C" fn shiika_internal_ptr_load(receiver: SkPtr) -> SkObj {
    SkObj::new(receiver.val() as *const ShiikaObject)
}

#[export_name = "Shiika::Internal::Ptr#store"]
pub extern "C" fn shiika_internal_ptr_store(receiver: SkPtr, object: SkObj) {
    unsafe {
        let p = (*receiver.0).value as *mut SkObj;
        //std::ptr::write(p, std::ptr::addr_of!(o));
        //*p = std::ptr::addr_of!(object);
        *p = object;
    }
}

#[export_name = "Shiika::Internal::Ptr#read"]
pub extern "C" fn shiika_internal_ptr_read(receiver: SkPtr) -> SkInt {
    unsafe {
        let b = std::ptr::read(receiver.val() as *const u8);
        (b as i64).into()
    }
}

#[export_name = "Shiika::Internal::Ptr#write"]
pub extern "C" fn shiika_internal_ptr_write(receiver: SkPtr, byte: SkInt) {
    unsafe {
        let p = (*receiver.0).value as *mut u8;
        *p = byte.val().try_into().unwrap();
    }
}
