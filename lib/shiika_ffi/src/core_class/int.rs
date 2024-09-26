//extern "C" {
//    fn shiika_intrinsic_box_int(i: i64) -> SkInt;
//}

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

impl SkInt {
    pub fn value(&self) -> i64 {
        unsafe { (*self.0).value }
    }
}
