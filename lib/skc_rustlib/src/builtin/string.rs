//! Instance of `::String`
use crate::builtin::{SkInt, SkPtr};
use std::ffi::CString;

extern "C" {
    // TODO: better name
    fn gen_literal_string(p: *const u8, bytesize: i64) -> SkStr;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkStr(*const ShiikaString);

#[repr(C)]
#[derive(Debug)]
struct ShiikaString {
    vtable: *const u8,
    class_obj: *const u8,
    ptr: SkPtr,
    bytesize: SkInt,
}

impl From<String> for SkStr {
    /// Make a Shiika `String` from Rust `String`. `s` must not contain a null byte in it.
    fn from(s: String) -> Self {
        let bytesize = s.as_bytes().len() as i64;
        let cstring = CString::new(s).unwrap();
        let leaked = Box::leak(Box::new(cstring));
        unsafe { gen_literal_string(leaked.as_ptr() as *const u8, bytesize) }
    }
}

impl SkStr {
    /// Returns byte slice
    // TODO: more Rust-y name?
    pub fn byteslice(&self) -> &[u8] {
        unsafe {
            let size = self.bytesize() as usize;
            std::slice::from_raw_parts(self.u8ptr(), size)
        }
    }

    fn u8ptr(&self) -> *const u8 {
        unsafe { (*self.0).ptr.unbox() }
    }

    fn bytesize(&self) -> i64 {
        unsafe { (*self.0).bytesize.val() }
    }
}
