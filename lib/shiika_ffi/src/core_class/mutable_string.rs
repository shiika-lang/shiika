#[repr(C)]
#[derive(Debug)]
pub struct SkMutableString(*mut ShiikaMutableString);

unsafe impl Send for SkMutableString {}

impl crate::SkValue for SkMutableString {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaMutableString {
    vtable: *const u8,
    class_obj: *const u8,
    value: *mut Vec<u8>,
}

impl SkMutableString {
    pub fn value(&self) -> &[u8] {
        unsafe { &*(*self.0).value }
    }

    pub fn value_mut(&self) -> &mut Vec<u8> {
        unsafe { &mut *(*self.0).value }
    }

    pub fn set_value(&self, bytes: Vec<u8>) {
        unsafe {
            (*self.0).value = Box::into_raw(Box::new(bytes));
        }
    }
}
