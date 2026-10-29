/// Shiika's `Int` is an unboxed immediate (llvm `i64`). `SkInt` is therefore a
/// plain value, not a pointer to a boxed object.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct SkInt(i64);

unsafe impl Send for SkInt {}

impl crate::SkValue for SkInt {
    fn as_raw_u64(self) -> u64 {
        self.0 as u64
    }
}

impl std::fmt::Display for SkInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val())
    }
}

impl From<SkInt> for i64 {
    fn from(sk_int: SkInt) -> Self {
        sk_int.0
    }
}

impl From<i64> for SkInt {
    fn from(i: i64) -> Self {
        SkInt(i)
    }
}

impl SkInt {
    pub fn val(&self) -> i64 {
        self.0
    }
}
