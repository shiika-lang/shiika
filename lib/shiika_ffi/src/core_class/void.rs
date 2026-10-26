extern "C" {
    fn shiika_intrinsic_box_bool(b: bool) -> *const u8;
}

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
        // `Void` is represented at runtime as a boxed `false` (matching the
        // compiler's `compile_void`), i.e. a pointer to a boxed Bool object.
        SkVoid(unsafe { shiika_intrinsic_box_bool(false) })
    }
}
