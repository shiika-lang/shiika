use crate::builtin::bool::SkBool;
use crate::builtin::string::SkString;
use std::io::Write;

/// A Shiika object
#[repr(C)]
#[derive(Debug)]
pub struct SkObj {
    vtable: *const u8,
    class_obj: *const u8,
}

#[export_name = "Object#=="]
pub extern "C" fn object_eq(receiver: *const u8, other: *const u8) -> SkBool {
    SkBool::new(receiver == other)
}

#[export_name = "Object#puts"]
pub extern "C" fn object_puts(receiver: *const u8, s: &SkString) {
    //TODO: Return SkVoid
    let _result = std::io::stdout().write_all(s.byteslice());
    println!("");
}
