use shiika_ffi::core_class::{SkBool, SkInt};
use shiika_ffi_macro::shiika_method;

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

//#[shiika_method("Int#/")]
//pub extern "C" fn int_div(receiver: SkInt, other: SkInt) -> SkFloat {
//    let a = receiver.val() as f64;
//    let b = other.val() as f64;
//    (a / b).into()
//}

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

//#[shiika_method("Int#to_f")]
//pub extern "C" fn int_to_f(receiver: SkInt) -> SkFloat {
//    (receiver.val() as f64).into()
//}
