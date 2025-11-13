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

impl SkString {
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
