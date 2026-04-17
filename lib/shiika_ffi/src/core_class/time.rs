use crate::core_class::SkInt;
use crate::core_class::SkObject;

#[repr(C)]
#[derive(Debug)]
pub struct SkTime(*mut ShiikaTime);

unsafe impl Send for SkTime {}

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

#[repr(C)]
#[derive(Debug)]
pub struct SkInstant(*mut ShiikaInstant);

unsafe impl Send for SkInstant {}

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

#[repr(C)]
#[derive(Debug)]
pub struct SkZone(*mut ShiikaZone);

unsafe impl Send for SkZone {}

extern "C" {
    #[allow(improper_ctypes)]
    static shiika_const_Time_Zone_Utc: SkObject;
    #[allow(improper_ctypes)]
    static shiika_const_Time_Zone_Local: SkObject;
}

#[repr(C)]
#[derive(Debug)]
struct ShiikaZone {
    vtable: *const u8,
    class_obj: *const u8,
}

impl SkZone {
    pub fn to_rs_zone(&self) -> RsZone {
        unsafe {
            let self_ptr = self.0 as *const u8;
            let utc_ptr = &shiika_const_Time_Zone_Utc as *const SkObject as *const *const u8;
            let local_ptr = &shiika_const_Time_Zone_Local as *const SkObject as *const *const u8;
            if self_ptr == *utc_ptr {
                RsZone::Utc
            } else if self_ptr == *local_ptr {
                RsZone::Local
            } else {
                panic!("SkZone::to_rs_zone failed");
            }
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct SkPlainDateTime(*mut ShiikaPlainDateTime);

unsafe impl Send for SkPlainDateTime {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaPlainDateTime {
    vtable: *const u8,
    class_obj: *const u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct SkPlainDate(*mut ShiikaPlainDate);

unsafe impl Send for SkPlainDate {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaPlainDate {
    vtable: *const u8,
    class_obj: *const u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct SkPlainTime(*mut ShiikaPlainTime);

unsafe impl Send for SkPlainTime {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaPlainTime {
    vtable: *const u8,
    class_obj: *const u8,
}

pub enum RsZone {
    Utc,
    Local,
}
