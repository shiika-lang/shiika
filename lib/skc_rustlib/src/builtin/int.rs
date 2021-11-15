/// Instance of `::Int`
/// May represent big number in the future
#[repr(C)]
#[derive(Debug)]
pub struct SkInt(*const ShiikaInt);

#[repr(C)]
#[derive(Debug)]
struct ShiikaInt {
    vtable: *const u8,
    class_obj: *const u8,
    value: i64,
}

impl SkInt {
    /// Convert to Rust value
    pub fn val(&self) -> i64 {
        unsafe { (*self.0).value }
    }
}
