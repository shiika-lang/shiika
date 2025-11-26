use crate::core_class::SkClass;
use shiika_ffi_macro::{shiika_const_ref, shiika_method_ref};

shiika_const_ref!("::String", SkClass, "sk_String");
shiika_method_ref!(
    "Meta:String#new",
    fn(receiver: SkClass, bytes: *const u8, n_bytes: u64) -> SkString,
    "meta_string_new"
);

#[repr(C)]
#[derive(Debug)]
pub struct SkString(*mut ShiikaString);

unsafe impl Send for SkString {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaString {
    vtable: *const u8,
    class_obj: *const u8,
    value: *mut Vec<u8>,
}

impl From<String> for SkString {
    /// Make a Shiika `String` from Rust `String`. `s` must not contain a null byte in it.
    fn from(s: String) -> Self {
        SkString::from_rust_string(s)
    }
}

impl SkString {
    pub fn from_rust_string(s_: impl Into<String>) -> SkString {
        let s = s_.into();
        meta_string_new(sk_String(), s.as_ptr(), s.len() as u64)
    }

    pub fn value(&self) -> &[u8] {
        unsafe { &*(*self.0).value }
    }

    pub fn set_value(&mut self, bytes: Vec<u8>) {
        unsafe {
            (*self.0).value = Box::into_raw(Box::new(bytes));
        }
    }

    /// Returns &str
    /// Panics if the content is invalid as utf-8
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.value()).unwrap()
    }
}
