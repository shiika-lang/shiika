use crate::core_class::SkClass;

#[repr(C)]
#[derive(Debug)]
pub struct SkObject(*const ShiikaObject);

unsafe impl Send for SkObject {}

impl SkObject {
    pub fn class(&self) -> SkClass {
        unsafe { (*self.0).class_obj.dup() }
    }

    /// Shallow clone
    pub fn dup(&self) -> SkObject {
        SkObject(self.0)
    }
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaObject {
    vtable: *const u8,
    class_obj: SkClass,
}
