#[repr(C)]
#[derive(Debug)]
pub struct SkPlainDateTime(*mut ShiikaPlainDateTime);
#[repr(C)]
#[derive(Debug)]
struct ShiikaPlainDateTime {
    vtable: *const u8,
    class_obj: *const u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct SkPlainDate(*mut ShiikaPlainDate);
#[repr(C)]
#[derive(Debug)]
struct ShiikaPlainDate {
    vtable: *const u8,
    class_obj: *const u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct SkPlainTime(*mut ShiikaPlainTime);
#[repr(C)]
#[derive(Debug)]
struct ShiikaPlainTime {
    vtable: *const u8,
    class_obj: *const u8,
}
