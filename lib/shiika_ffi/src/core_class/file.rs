use crate::core_class::SkString;
use std::fs::File;
use std::io::BufReader;

#[repr(C)]
#[derive(Debug)]
pub struct SkFile(pub *mut ShiikaFile);

unsafe impl Send for SkFile {}

impl crate::SkValue for SkFile {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaFile {
    pub vtable: *const u8,
    pub class_obj: *const u8,
    pub path: SkString,
    /// Raw pointer to a heap-allocated BufReader<File>.
    /// Stored in the slot of Shiika ivar `@file: Object`.
    pub buf_reader_ptr: *mut BufReader<File>,
}

impl SkFile {
    /// Mutable reference to the BufReader stored in `@file`.
    pub fn buf_reader_mut(&mut self) -> &mut BufReader<File> {
        unsafe { &mut *(*self.0).buf_reader_ptr }
    }

    pub fn set_buf_reader(&mut self, br: BufReader<File>) {
        unsafe {
            (*self.0).buf_reader_ptr = Box::into_raw(Box::new(br));
        }
    }

    pub fn buf_reader_raw(&self) -> *mut BufReader<File> {
        unsafe { (*self.0).buf_reader_ptr }
    }
}
