use crate::builtin::time::rs_zone::RsZone;
use crate::builtin::SkObj;

#[repr(C)]
#[derive(Debug)]
pub struct SkZone(*mut ShiikaZone);

extern "C" {
    #[allow(improper_ctypes)]
    static shiika_const_Time_Zone_Utc: SkObj;
    #[allow(improper_ctypes)]
    static shiika_const_Time_Zone_Local: SkObj;
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaZone {
    vtable: *const u8,
    class_obj: SkObj,
}

impl SkZone {
    // Maybe there should be a macro to do this conversion.
    pub fn to_rs_zone(&self) -> RsZone {
        unsafe {
            if shiika_const_Time_Zone_Utc.same_object(self.0) {
                RsZone::Utc
            } else if shiika_const_Time_Zone_Local.same_object(self.0) {
                RsZone::Local
            } else {
                panic!("SkZone::to_rs_zone failed");
            }
        }
    }
}
