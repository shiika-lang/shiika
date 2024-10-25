extern "C" {
    fn shiika_intrinsic_box_int(i: i64) -> SkInt;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkInt(*const ShiikaInt);

unsafe impl Send for SkInt {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaInt {
    vtable: *const u8,
    class_obj: *const u8,
    value: i64,
}

impl From<SkInt> for i64 {
    fn from(sk_int: SkInt) -> Self {
        unsafe { (*sk_int.0).value }
    }
}

impl From<i64> for SkInt {
    fn from(i: i64) -> Self {
        unsafe { shiika_intrinsic_box_int(i) }
    }
}

impl SkInt {
    pub fn val(&self) -> i64 {
        unsafe { (*self.0).value }
    }
}
