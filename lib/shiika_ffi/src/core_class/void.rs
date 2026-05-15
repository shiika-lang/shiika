use crate::core_class::SkBool;

#[repr(C)]
#[derive(Debug)]
pub struct SkVoid(pub *const u8);

unsafe impl Send for SkVoid {}

impl crate::SkValue for SkVoid {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

impl From<()> for SkVoid {
    fn from(_: ()) -> Self {
        let b: SkBool = false.into();
        SkVoid(b.0 as *const u8)
    }
}
