/// Instance of `::String`
use crate::builtin::int::SkInt;
use crate::builtin::shiika_internal_ptr::SkPtr;

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
        unsafe { (*self.0).ptr.val() }
    }

    fn bytesize(&self) -> i64 {
        unsafe { (*self.0).bytesize.val() }
    }
}
