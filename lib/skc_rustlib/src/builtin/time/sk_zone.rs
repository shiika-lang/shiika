#[repr(C)]
#[derive(Debug)]
pub struct SkZone(*mut ShiikaZone);

#[repr(C)]
#[derive(Debug)]
struct ShiikaZone {
    vtable: *const u8,
    class_obj: *const u8,
}

impl SkZone {
    pub fn local() -> SkZone {}

    fn to_rs_zone(&self) -> RsZone {}
}
