use crate::builtin::bool::SkBool;
use crate::builtin::class::{ShiikaClass, SkClass};
use crate::builtin::string::SkStr;
use std::io::Write;
#[repr(C)]
#[derive(Debug)]
pub struct SkObj(*const ShiikaObject);

/// A Shiika object
#[repr(C)]
#[derive(Debug)]
struct ShiikaObject {
    vtable: *const u8,
    class_obj: *const ShiikaClass,
}

impl SkObj {
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

#[export_name = "Object#puts"]
pub extern "C" fn object_puts(_receiver: *const u8, s: SkStr) {
    //TODO: Return SkVoid
    let _result = std::io::stdout().write_all(s.byteslice());
    println!("");
}
