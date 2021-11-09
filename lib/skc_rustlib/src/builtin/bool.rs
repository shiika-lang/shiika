extern "C" {
    fn box_bool(b: bool) -> SkBool;
    //fn unbox_bool(b: SkBool) -> bool;
}

/// An instance of `Bool`
#[repr(C)]
#[derive(Debug)]
pub struct SkBool {
    vtable: *const u8,
    class_obj: *const u8,
    value: bool,
}

impl SkBool {
    /// Make Shiika bool from Rust bool
    pub fn new(b: bool) -> SkBool {
        unsafe { box_bool(b) }
    }

    // Convert to Rust value
    //    pub fn val(&self) -> bool {
    //        self.value
    //    }
}
