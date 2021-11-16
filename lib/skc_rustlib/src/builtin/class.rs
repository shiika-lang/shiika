/// An instance of `::Class`
use crate::builtin::string::SkStr;
#[repr(C)]
#[derive(Debug)]
pub struct SkClass(*const ShiikaClass);

impl SkClass {
    pub fn new(ptr: *const ShiikaClass) -> SkClass {
        SkClass(ptr)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaClass {
    vtable: *const u8,
    name: SkStr,
}
