extern "C" {
    fn shiika_intrinsic_box_float(f: f64) -> SkFloat;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkFloat(*const ShiikaFloat);

unsafe impl Send for SkFloat {}

impl crate::SkValue for SkFloat {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

impl std::fmt::Display for SkFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val())
    }
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaFloat {
    vtable: *const u8,
    class_obj: *const u8,
    value: f64,
}

impl From<SkFloat> for f64 {
    fn from(sk_float: SkFloat) -> Self {
        unsafe { (*sk_float.0).value }
    }
}

impl From<f64> for SkFloat {
    fn from(f: f64) -> Self {
        unsafe { shiika_intrinsic_box_float(f) }
    }
}

impl SkFloat {
    pub fn val(&self) -> f64 {
        unsafe { (*self.0).value }
    }
}
