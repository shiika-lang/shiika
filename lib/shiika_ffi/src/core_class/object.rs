use crate::core_class::SkClass;

#[repr(C)]
#[derive(Debug)]
pub struct SkObject(*const ShiikaObject);

unsafe impl Send for SkObject {}

impl SkObject {
    pub fn class(&self) -> SkClass {
        unsafe { (*self.0).class_obj.dup() }
    }
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaObject {
    vtable: *const u8,
    class_obj: SkClass,
}
