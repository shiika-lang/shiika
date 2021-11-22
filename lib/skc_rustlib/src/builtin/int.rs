/// Instance of `::Int`
/// May represent big number in the future
use crate::builtin::SkBool;

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

impl From<i64> for SkInt {
    fn from(i: i64) -> Self {
        unsafe { box_int(i) }
    }
}

impl SkInt {
    /// Convert to Rust value
    pub fn val(&self) -> i64 {
        unsafe { (*self.0).value }
    }
}

#[export_name = "Int#+"]
pub extern "C" fn int_add(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() + other.val()).into()
}

#[export_name = "Int#-"]
pub extern "C" fn int_sub(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() + other.val()).into()
}

#[export_name = "Int#*"]
pub extern "C" fn int_mul(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() * other.val()).into()
}

//#[export_name = "Int#/"]
//pub extern "C" fn int_add(receiver: SkInt, other: SkInt) -> SkFloat {
//    (receiver.val() + other.val()).into()
//}

#[export_name = "Int#%"]
pub extern "C" fn int_mod(receiver: SkInt, other: SkInt) -> SkInt {
    (receiver.val() % other.val()).into()
}

#[export_name = "Int#<"]
pub extern "C" fn int_lt(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() < other.val()).into()
}

#[export_name = "Int#<="]
pub extern "C" fn int_le(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() <= other.val()).into()
}

#[export_name = "Int#>"]
pub extern "C" fn int_gt(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() > other.val()).into()
}

#[export_name = "Int#>="]
pub extern "C" fn int_ge(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() >= other.val()).into()
}

#[export_name = "Int#=="]
pub extern "C" fn int_eq(receiver: SkInt, other: SkInt) -> SkBool {
    (receiver.val() == other.val()).into()
}
