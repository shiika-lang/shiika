use crate::builtin::class::SkModule;
use crate::builtin::{SkBool, SkInt, SkStr};
use plain::Plain;
use shiika_ffi_macro::shiika_method;
use std::io::{stdout, Write};
use std::mem;

#[repr(C)]
#[derive(Debug)]
pub struct SkObj(*const ShiikaObject);

unsafe impl Plain for SkObj {}

/// A Shiika object
#[repr(C)]
#[derive(Debug)]
pub struct ShiikaObject {
    vtable: *const u8,
    class_obj: SkModule,
}

impl SkObj {
    //    pub fn new(p: *const ShiikaObject) -> SkObj {
    //        SkObj(p)
    //    }

    pub fn class(&self) -> SkModule {
        unsafe { (*self.0).class_obj.dup() }
    }
}

#[shiika_method("Object#==")]
pub extern "C" fn object_eq(receiver: *const u8, other: *const u8) -> SkBool {
    (receiver == other).into()
}

#[shiika_method("Object#class")]
pub extern "C" fn object_class(receiver: SkObj) -> SkModule {
    receiver.class()
}

// TODO: Move to `Process.exit` or something
#[shiika_method("Object#exit")]
pub extern "C" fn object_exit(_receiver: SkObj, code: SkInt) {
    std::process::exit(code.val() as i32);
}

#[shiika_method("Object#object_id")]
pub extern "C" fn object_object_id(receiver: SkObj) -> SkInt {
    unsafe {
        let i = mem::transmute::<*const ShiikaObject, i64>(receiver.0);
        i.into()
    }
}

#[shiika_method("Object#print")]
pub extern "C" fn object_print(_receiver: *const u8, s: SkStr) {
    //TODO: Return SkVoid
    let _ = stdout().write_all(s.as_byteslice());
    let _ = stdout().flush();
}

#[shiika_method("Object#puts")]
pub extern "C" fn object_puts(_receiver: *const u8, s: SkStr) {
    //TODO: Return SkVoid
    let _ = stdout().write_all(s.as_byteslice());
    println!("");
}
