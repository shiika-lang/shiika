/// Shiika's `Float` is an unboxed immediate (llvm `double`). `SkFloat` is therefore a
/// plain value, not a pointer to a boxed object.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct SkFloat(f64);

unsafe impl Send for SkFloat {}

impl crate::SkValue for SkFloat {
    fn as_raw_u64(self) -> u64 {
        self.0.to_bits()
    }
}

impl std::fmt::Display for SkFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val())
    }
}

impl From<SkFloat> for f64 {
    fn from(sk_float: SkFloat) -> Self {
        sk_float.0
    }
}

impl From<f64> for SkFloat {
    fn from(f: f64) -> Self {
        SkFloat(f)
    }
}

impl SkFloat {
    pub fn val(&self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SkFloat;
    use crate::SkValue;

    #[test]
    fn preserves_float_bits() {
        for bits in [
            0,
            0x8000_0000_0000_0000,
            0x3ff8_0000_0000_0000,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x7ff8_0000_0000_1234,
        ] {
            let value = SkFloat::from(f64::from_bits(bits));
            assert_eq!(value.val().to_bits(), bits);
            assert_eq!(f64::from(value).to_bits(), bits);
            assert_eq!(value.as_raw_u64(), bits);
        }
    }
}
