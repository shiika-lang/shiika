/// Instance of `::Shiika::Internal::Ptr`
#[repr(C)]
#[derive(Debug)]
pub struct SkPtr(*const ShiikaPtr);

#[repr(C)]
#[derive(Debug)]
struct ShiikaPtr {
    vtable: *const u8,
    class_obj: *const u8,
    value: *const u8,
}

impl SkPtr {
    /// Convert to Rust value
    pub fn val(&self) -> *const u8 {
        unsafe { (*self.0).value }
    }
}
