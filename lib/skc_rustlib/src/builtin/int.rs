//! Instance of `::Int`
//! May represent big number in the future
use crate::builtin::{SkBool, SkFloat};
use shiika_ffi_macro::shiika_method;
use std::fmt;

extern "C" {
    fn box_int(i: i64) -> SkInt;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkInt(*const ShiikaInt);

#[repr(C)]
#[derive(Debug)]
struct ShiikaInt {
    vtable: *const u8,
    class_obj: *const u8,
    value: i64,
}

impl From<SkInt> for i64 {
    fn from(sk_int: SkInt) -> Self {
        unsafe { (*sk_int.0).value }
    }
}

impl From<SkInt> for u64 {
    fn from(sk_int: SkInt) -> Self {
        unsafe { (*sk_int.0).value as u64 }
    }
}

impl From<SkInt> for usize {
    fn from(sk_int: SkInt) -> Self {
        unsafe { (*sk_int.0).value as usize }
    }
}

impl From<i64> for SkInt {
    fn from(i: i64) -> Self {
        unsafe { box_int(i) }
    }
}

impl From<i32> for SkInt {
    fn from(i: i32) -> Self {
        unsafe { box_int(i as i64) }
    }
}

impl From<u32> for SkInt {
    fn from(i: u32) -> Self {
        unsafe { box_int(i as i64) }
    }
}

impl From<usize> for SkInt {
    fn from(i: usize) -> Self {
        unsafe { box_int(i as i64) }
    }
}

impl fmt::Display for SkInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.val())
    }
}

impl SkInt {
    pub fn new(i: i64) -> SkInt {
        i.into()
    }

    /// Convert to Rust value
    pub fn val(&self) -> i64 {
        unsafe { (*self.0).value }
    }
}

#[shiika_method("Int#-@")]
pub extern "C" fn int_inv(receiver: SkInt) -> SkInt {
    (-receiver.val()).into()
}

#[shiika_method("Int#+")]
pub extern "C" fn int_add(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() + other.val()).into()
}

#[shiika_method("Int#-")]
pub extern "C" fn int_sub(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() - other.val()).into()
}

#[shiika_method("Int#*")]
pub extern "C" fn int_mul(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() * other.val()).into()
}

#[shiika_method("Int#/")]
pub extern "C" fn int_div(receiver: SkInt, other: SkInt) -> SkFloat {
    let a = receiver.val() as f64;
    let b = other.val() as f64;
    (a / b).into()
}

#[shiika_method("Int#%")]
pub extern "C" fn int_mod(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() % other.val()).into()
}

#[shiika_method("Int#and")]
pub extern "C" fn int_and(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() & other.val()).into()
}

#[shiika_method("Int#or")]
pub extern "C" fn int_or(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() | other.val()).into()
}

#[shiika_method("Int#xor")]
pub extern "C" fn int_xor(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() ^ other.val()).into()
}

#[shiika_method("Int#lshift")]
pub extern "C" fn int_lshift(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() << other.val()).into()
}

#[shiika_method("Int#rshift")]
pub extern "C" fn int_rshift(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() >> other.val()).into()
}

#[shiika_method("Int#<")]
pub extern "C" fn int_lt(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() < other.val()).into()
}

#[shiika_method("Int#<=")]
pub extern "C" fn int_le(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() <= other.val()).into()
}

#[shiika_method("Int#>")]
pub extern "C" fn int_gt(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() > other.val()).into()
}

#[shiika_method("Int#>=")]
pub extern "C" fn int_ge(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() >= other.val()).into()
}

#[shiika_method("Int#==")]
pub extern "C" fn int_eq(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() == other.val()).into()
}

#[shiika_method("Int#to_f")]
pub extern "C" fn int_to_f(receiver: SkInt) -> SkFloat {
    (receiver.val() as f64).into()
}
