/// Shiika's `Bool` is an unboxed immediate (llvm `i1`, C ABI `bool`/i8).
/// `SkBool` is therefore a plain value, not a pointer to a boxed object.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct SkBool(bool);

unsafe impl Send for SkBool {}

impl crate::SkValue for SkBool {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

impl From<SkBool> for bool {
    fn from(sk_bool: SkBool) -> Self {
        sk_bool.0
    }
}

impl From<bool> for SkBool {
    fn from(b: bool) -> Self {
        SkBool(b)
    }
}
