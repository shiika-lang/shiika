use crate::builtin::class::SkClass;
use crate::builtin::{SkBool, SkFloat, SkInt, SkResult, SkStr};
use plain::Plain;
use shiika_ffi_macro::shiika_method;
use std::fmt;
use std::io::{stdin, stdout, Write};
use std::mem;
use std::thread;
use std::time;

#[repr(C)]
pub struct SkObj(*const ShiikaObject);

impl fmt::Debug for SkObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_null() {
            f.write_str("SkObj { !null! }")
        } else {
            f.debug_struct("SkObj")
                .field("class_obj", &self.class())
                .finish()
        }
    }
}

unsafe impl Plain for SkObj {}

/// A Shiika object
#[repr(C)]
#[derive(Debug)]
pub struct ShiikaObject {
    vtable: *const u8,
    class_obj: SkClass,
}

impl SkObj {
    pub fn new(p: *const ShiikaObject) -> SkObj {
        SkObj(p)
    }

    /// Shallow clone
    pub fn dup(&self) -> SkObj {
        SkObj(self.0)
    }

    pub fn class(&self) -> SkClass {
        unsafe { (*self.0).class_obj.dup() }
    }

    pub fn same_object<T>(&self, other: *const T) -> bool {
        self.0 == (other as *const ShiikaObject)
    }
}

#[shiika_method("Object#==")]
pub extern "C" fn object_eq(receiver: *const u8, other: *const u8) -> SkBool {
    (receiver == other).into()
}

#[shiika_method("Object#class")]
pub extern "C" fn object_class(receiver: SkObj) -> SkClass {
    receiver.class()
}

// TODO: Move to `Process.exit` or something
#[shiika_method("Object#exit")]
pub extern "C" fn object_exit(_receiver: SkObj, code: SkInt) {
    std::process::exit(code.val() as i32);
}

#[shiika_method("Object#gets")]
pub extern "C" fn object_gets(_receiver: *const u8) -> SkResult<SkStr> {
    let mut buffer = String::new();
    stdin()
        .read_line(&mut buffer)
        .map(|_| SkStr::new(buffer))
        .into()
}

#[shiika_method("Object#object_id")]
pub extern "C" fn object_object_id(receiver: SkObj) -> SkInt {
    unsafe {
        let i = mem::transmute::<*const ShiikaObject, i64>(receiver.0);
        i.into()
    }
}

#[shiika_method("Object#panic")]
pub extern "C" fn object_panic(_receiver: *const u8, s: SkStr) {
    panic!("{}", s.as_str());
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

#[shiika_method("Object#sleep")]
pub extern "C" fn object_sleep(_receiver: *const u8, sec: SkFloat) {
    //TODO: Return SkVoid
    thread::sleep(time::Duration::from_secs_f64(sec.into()));
}
