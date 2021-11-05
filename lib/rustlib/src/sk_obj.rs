#[repr(C)]
#[derive(Debug)]
pub struct SkInt {
    vtable: *const u8,
    class_obj: *const u8,
    value: i64,
}

#[repr(C)]
#[derive(Debug)]
pub struct SkPtr {
    vtable: *const u8,
    class_obj: *const u8,
    value: *const u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct SkString {
    vtable: *const u8,
    class_obj: *const u8,
    ptr: *const SkPtr,
    bytesize: *const SkInt,
}

impl SkString {
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            let ptr = (*self.ptr).value;
            let size = (*self.bytesize).value as usize;
            std::slice::from_raw_parts(ptr, size)
        }
    }
}
