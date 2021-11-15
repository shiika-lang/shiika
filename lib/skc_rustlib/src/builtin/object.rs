use crate::builtin::bool::SkBool;
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
    class_obj: *const u8,
}

#[export_name = "Object#=="]
pub extern "C" fn object_eq(receiver: *const u8, other: *const u8) -> SkBool {
    (receiver == other).into()
}

#[export_name = "Object#puts"]
pub extern "C" fn object_puts(_receiver: *const u8, s: SkStr) {
    //TODO: Return SkVoid
    let _result = std::io::stdout().write_all(s.byteslice());
    println!("");
}
