use crate::builtin::SkInt;

#[repr(C)]
#[derive(Debug)]
pub struct SkInstant(*mut ShiikaInstant);

#[repr(C)]
#[derive(Debug)]
struct ShiikaInstant {
    vtable: *const u8,
    class_obj: *const u8,
    nano_timestamp: SkInt,
}

impl SkInstant {
    pub fn nano_timestamp(&self) -> i64 {
        unsafe { (*self.0).nano_timestamp.val() }
    }
}
