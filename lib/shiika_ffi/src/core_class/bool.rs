extern "C" {
    fn shiika_intrinsic_box_bool(b: bool) -> SkBool;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkBool(*const ShiikaBool);

unsafe impl Send for SkBool {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaBool {
    vtable: *const u8,
    class_obj: *const u8,
    value: bool,
}

impl From<SkBool> for bool {
    fn from(sk_bool: SkBool) -> Self {
        unsafe { (*sk_bool.0).value }
    }
}

impl From<bool> for SkBool {
    fn from(b: bool) -> Self {
        unsafe { shiika_intrinsic_box_bool(b) }
    }
}
