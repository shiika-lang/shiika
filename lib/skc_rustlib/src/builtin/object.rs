use crate::builtin::class::{ShiikaClass, SkClass};
use crate::builtin::{SkBool, SkInt, SkStr};
use plain::Plain;
use std::io::Write;
#[repr(C)]
#[derive(Debug)]
pub struct SkObj(*const ShiikaObject);

unsafe impl Plain for SkObj {}

/// A Shiika object
#[repr(C)]
#[derive(Debug)]
pub struct ShiikaObject {
    vtable: *const u8,
    class_obj: *const ShiikaClass,
}

impl SkObj {
    pub fn new(p: *const ShiikaObject) -> SkObj {
        SkObj(p)
    }

    //    pub fn raw(&self) -> *const ShiikaObject {
    //        self.0
    //    }

    pub fn dup_ptr(&self) -> SkObj {
        SkObj(self.0)
    }

    pub fn class(&self) -> SkClass {
        unsafe { SkClass::new((*self.0).class_obj) }
    }
}

#[export_name = "Object#=="]
pub extern "C" fn object_eq(receiver: *const u8, other: *const u8) -> SkBool {
    (receiver == other).into()
}

#[export_name = "Object#class"]
pub extern "C" fn object_class(receiver: SkObj) -> SkClass {
    receiver.class()
}

// TODO: Move to `Process.exit` or something
#[export_name = "Object#exit"]
pub extern "C" fn object_exit(_receiver: SkObj, code: SkInt) {
    std::process::exit(code.val() as i32);
}

#[export_name = "Object#puts"]
pub extern "C" fn object_puts(_receiver: *const u8, s: SkStr) {
    //TODO: Return SkVoid
    let _result = std::io::stdout().write_all(s.byteslice());
    println!("");
}