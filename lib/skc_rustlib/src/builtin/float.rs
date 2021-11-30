//! Instance of `::Float`
use crate::builtin::{SkBool, SkInt, SkStr};

extern "C" {
    fn box_float(f: f64) -> SkFloat;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkFloat(*const ShiikaFloat);

#[repr(C)]
#[derive(Debug)]
struct ShiikaFloat {
    vtable: *const u8,
    class_obj: *const u8,
    value: f64,
}

impl From<SkFloat> for f64 {
    fn from(sk_float: SkFloat) -> Self {
        unsafe { (*sk_float.0).value }
    }
}

impl From<f64> for SkFloat {
    fn from(f: f64) -> Self {
        unsafe { box_float(f) }
    }
}

impl SkFloat {
    /// Convert to Rust value
    pub fn val(&self) -> f64 {
        unsafe { (*self.0).value }
    }
}

#[export_name = "Float#-@"]
pub extern "C" fn float_inv(receiver: SkFloat) -> SkFloat {
    (-receiver.val()).into()
}

#[export_name = "Float#+"]
pub extern "C" fn float_add(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() + other.val()).into()
}

#[export_name = "Float#-"]
pub extern "C" fn float_sub(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() - other.val()).into()
}

#[export_name = "Float#*"]
pub extern "C" fn float_mul(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() * other.val()).into()
}

#[export_name = "Float#/"]
pub extern "C" fn float_div(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() / other.val()).into()
}

#[export_name = "Float#<"]
pub extern "C" fn float_lt(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() < other.val()).into()
}

#[export_name = "Float#<="]
pub extern "C" fn float_le(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() <= other.val()).into()
}

#[export_name = "Float#>"]
pub extern "C" fn float_gt(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() > other.val()).into()
}

#[export_name = "Float#>="]
pub extern "C" fn float_ge(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() >= other.val()).into()
}

#[export_name = "Float#=="]
pub extern "C" fn float_eq(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() == other.val()).into()
}

#[export_name = "Float#abs"]
pub extern "C" fn float_abs(receiver: SkFloat) -> SkFloat {
    receiver.val().abs().into()
}

#[export_name = "Float#floor"]
pub extern "C" fn float_floor(receiver: SkFloat) -> SkFloat {
    receiver.val().floor().into()
}

#[export_name = "Float#to_i"]
pub extern "C" fn float_to_i(receiver: SkFloat) -> SkInt {
    (receiver.val().trunc() as i64).into()
}

#[export_name = "Float#to_s"]
pub extern "C" fn float_to_s(receiver: SkFloat) -> SkStr {
    format!("{}", receiver.val()).into()
}
