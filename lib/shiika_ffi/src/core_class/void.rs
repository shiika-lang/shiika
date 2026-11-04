use shiika_ffi_macro::shiika_const_ref;

shiika_const_ref!("::Void", SkVoid, "sk_Void");

#[repr(C)]
#[derive(Debug)]
pub struct SkVoid(pub *const u8);

unsafe impl Send for SkVoid {}

impl crate::SkValue for SkVoid {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

impl SkVoid {
    pub fn dup(&self) -> SkVoid {
        SkVoid(self.0)
    }
}

impl From<()> for SkVoid {
    fn from(_: ()) -> Self {
        // `Void` is a singleton: return the `::Void` instance (matching the
        // compiler's `compile_void`) rather than fabricating a new object.
        sk_Void()
    }
}
