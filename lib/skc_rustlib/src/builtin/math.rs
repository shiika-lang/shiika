use crate::builtin::SkFloat;
use shiika_ffi_macro::shiika_method;

#[shiika_method("Meta:Math#sin")]
pub extern "C" fn math_sin(_receiver: *const u8, x: SkFloat) -> SkFloat {
    x.val().sin().into()
}

#[shiika_method("Meta:Math#cos")]
pub extern "C" fn math_cos(_receiver: *const u8, x: SkFloat) -> SkFloat {
    x.val().cos().into()
}

#[shiika_method("Meta:Math#sqrt")]
pub extern "C" fn math_sqrt(_receiver: *const u8, x: SkFloat) -> SkFloat {
    x.val().sqrt().into()
}
