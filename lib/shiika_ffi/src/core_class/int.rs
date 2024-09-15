//extern "C" {
//    fn shiika_intrinsic_box_int(i: i64) -> SkInt;
//}

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
