//! Instance of `::String`
use crate::builtin::object::ShiikaObject;
use crate::builtin::{SkAry, SkInt, SkObj, SkPtr};
use shiika_ffi_macro::shiika_method;
use std::ffi::CString;
use unicode_segmentation::UnicodeSegmentation;

extern "C" {
    // TODO: better name
    fn gen_literal_string(p: *const u8, bytesize: i64) -> SkStr;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkStr(*const ShiikaString);

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaString {
    vtable: *const u8,
    class_obj: *const u8,
    ptr: SkPtr,
    bytesize: SkInt,
}

impl From<String> for SkStr {
    /// Make a Shiika `String` from Rust `String`. `s` must not contain a null byte in it.
    fn from(s: String) -> Self {
        SkStr::new(s)
    }
}

impl From<SkStr> for SkObj {
    fn from(s: SkStr) -> SkObj {
        SkObj::new(s.0 as *const ShiikaObject)
    }
}

impl SkStr {
    pub fn new(s_: impl Into<String>) -> SkStr {
        let s = s_.into();
        let bytesize = s.as_bytes().len() as i64;
        let cstring = CString::new(s).unwrap();
        let leaked = Box::leak(Box::new(cstring));
        unsafe { gen_literal_string(leaked.as_ptr() as *const u8, bytesize) }
    }
}

impl SkStr {
    /// Shallow clone
    pub fn dup(&self) -> SkStr {
        SkStr(self.0)
    }

    fn u8ptr(&self) -> *const u8 {
        unsafe { (*self.0).ptr.unbox() }
    }

    fn bytesize(&self) -> i64 {
        unsafe { (*self.0).bytesize.val() }
    }

    /// Returns byte slice
    pub fn as_byteslice(&self) -> &[u8] {
        unsafe {
            let size = self.bytesize() as usize;
            std::slice::from_raw_parts(self.u8ptr(), size)
        }
    }

    /// Returns &str
    /// Panics if the content is invalid as utf-8
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_byteslice()).unwrap()
    }
}

#[shiika_method("String#chars")]
pub extern "C" fn string_chars(receiver: SkStr) -> SkAry<SkStr> {
    let ary = SkAry::<SkStr>::new();
    let v = UnicodeSegmentation::graphemes(receiver.as_str(), true)
        .map(|s| s.to_string().into())
        .collect::<Vec<SkStr>>();
    ary.set_vec(v);
    ary
}

// TODO: How to support `break`
//#[shiika_method("String#each_char")]
//pub extern "C" fn string_each_char(receiver: SkStr, block: SkFn1<SkStr, SkVoid>) {
//    UnicodeSegmentation::graphemes(receiver.as_str(), true)
//        .map(|s| s.to_string().into())
//        .for_each(|sk_str| {
//            block.call(sk_str);
//        });
//}
