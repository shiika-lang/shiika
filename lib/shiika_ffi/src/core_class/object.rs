#[repr(C)]
#[derive(Debug)]
pub struct SkObject(*const ShiikaObject);

unsafe impl Send for SkObject {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaObject {
    vtable: *const u8,
    class_obj: *const u8,
}
