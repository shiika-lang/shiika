use shiika_ffi::core_class::{SkBool, SkFloat, SkInt, SkString};
use shiika_ffi_macro::{async_shiika_method, shiika_method};

#[shiika_method("Float#-@")]
pub extern "C" fn float_uminus(receiver: SkFloat) -> SkFloat {
    (-receiver.val()).into()
}

#[shiika_method("Float#+")]
pub extern "C" fn float_add(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() + other.val()).into()
}

#[shiika_method("Float#-")]
pub extern "C" fn float_sub(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() - other.val()).into()
}

#[shiika_method("Float#*")]
pub extern "C" fn float_mul(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() * other.val()).into()
}

#[shiika_method("Float#/")]
pub extern "C" fn float_div(receiver: SkFloat, other: SkFloat) -> SkFloat {
    (receiver.val() / other.val()).into()
}

#[async_shiika_method("Float#<")]
async fn float_lt(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() < other.val()).into()
}

#[async_shiika_method("Float#<=")]
async fn float_le(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() <= other.val()).into()
}

#[async_shiika_method("Float#>")]
async fn float_gt(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() > other.val()).into()
}

#[async_shiika_method("Float#>=")]
async fn float_ge(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() >= other.val()).into()
}

#[async_shiika_method("Float#==")]
async fn float_eq(receiver: SkFloat, other: SkFloat) -> SkBool {
    (receiver.val() == other.val()).into()
}

#[shiika_method("Float#abs")]
pub extern "C" fn float_abs(receiver: SkFloat) -> SkFloat {
    receiver.val().abs().into()
}

#[shiika_method("Float#floor")]
pub extern "C" fn float_floor(receiver: SkFloat) -> SkFloat {
    receiver.val().floor().into()
}

#[shiika_method("Float#to_i")]
pub extern "C" fn float_to_i(receiver: SkFloat) -> SkInt {
    (receiver.val().trunc() as i64).into()
}

#[async_shiika_method("Float#to_s")]
async fn float_to_s(receiver: SkFloat) -> SkString {
    format!("{}", receiver.val()).into()
}
