use crate::builtin::time::rs_zone::RsZone;
use crate::builtin::time::sk_instant::SkInstant;
use crate::builtin::time::SkZone;

#[repr(C)]
#[derive(Debug)]
pub struct SkTime(*mut ShiikaTime);

#[repr(C)]
#[derive(Debug)]
struct ShiikaTime {
    vtable: *const u8,
    class_obj: *const u8,
    instant: SkInstant,
    zone: SkZone,
}

impl SkTime {
    pub fn epoch(&self) -> i64 {
        let sk_instant = unsafe { &(*self.0).instant };
        sk_instant.nano_timestamp()
    }

    pub fn zone(&self) -> RsZone {
        unsafe { (*self.0).zone.to_rs_zone() }
    }
}
