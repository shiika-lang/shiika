#[repr(C)]
#[derive(Debug)]
pub struct SkVoid(*const ShiikaVoid);

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaVoid {
    vtable: *const u8,
    class_obj: *const u8,
}
