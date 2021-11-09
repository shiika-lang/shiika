/// Instance of `::Int`
#[repr(C)]
#[derive(Debug)]
pub struct SkInt {
    vtable: *const u8,
    class_obj: *const u8,
    value: i64,
}

impl SkInt {
    /// Convert to Rust value
    pub fn val(&self) -> i64 {
        self.value
    }
}
