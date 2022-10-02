#[repr(C)]
#[derive(Debug)]
pub struct SkTime(*mut ShiikaTime);

#[repr(C)]
#[derive(Debug)]
struct ShiikaTime {
    vtable: *const u8,
    class_obj: *const u8,
}

impl ShiikaTime {}
