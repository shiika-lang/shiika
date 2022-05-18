//! An instance of `Bool`. Interchangable to Rust bool via `into`.
//!
//! # Example
//!
//! ```rust
//! let b = true;
//! let sk_bool: SkBool = b.into();
//! let rust_bool: bool = sk_bool.into();
//! ```

extern "C" {
    fn box_bool(b: bool) -> SkBool;
}

#[repr(C)]
pub struct SkBool(*const ShiikaBool);

#[repr(C)]
#[derive(Debug)]
struct ShiikaBool {
    vtable: *const u8,
    class_obj: *const u8,
    value: bool,
}

impl From<SkBool> for bool {
    fn from(sk_bool: SkBool) -> Self {
        unsafe { (*sk_bool.0).value }
    }
}

impl From<bool> for SkBool {
    fn from(b: bool) -> Self {
        unsafe { box_bool(b) }
    }
}
