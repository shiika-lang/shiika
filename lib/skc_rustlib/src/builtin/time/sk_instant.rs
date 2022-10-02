#[repr(C)]
#[derive(Debug)]
pub struct SkInstant(*mut ShiikaInstant);

#[repr(C)]
#[derive(Debug)]
struct ShiikaInstant {
    vtable: *const u8,
    class_obj: *const u8,
}
