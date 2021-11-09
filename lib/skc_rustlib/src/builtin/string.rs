use crate::builtin::int::SkInt;
use crate::builtin::ptr::SkPtr;

/// Instance of `::String`
#[repr(C)]
#[derive(Debug)]
pub struct SkString {
    vtable: *const u8,
    class_obj: *const u8,
    ptr: *const SkPtr,
    bytesize: *const SkInt,
}

impl SkString {
    /// Returns byte slice
    pub fn byteslice(&self) -> &[u8] {
        unsafe {
            let ptr = (*self.ptr).val();
            let size = (*self.bytesize).val() as usize;
            std::slice::from_raw_parts(ptr, size)
        }
    }
}
