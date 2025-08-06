#[repr(C)]
#[derive(Debug)]
pub struct SkString(*mut ShiikaString);

unsafe impl Send for SkString {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaString {
    vtable: *const u8,
    class_obj: *const u8,
    value: Vec<u8>,
}

impl SkString {
    pub fn value(&self) -> &[u8] {
        unsafe { &(*self.0).value }
    }

    pub fn set_value(&mut self, bytes: &[u8]) {
        unsafe {
            (*self.0).value = bytes.to_vec();
        }
    }
}
