use crate::builtin::SkFloat;

#[export_name = "Meta:Math#sin"]
pub extern "C" fn math_sin(_receiver: *const u8, x: SkFloat) -> SkFloat {
    x.val().sin().into()
}

#[export_name = "Meta:Math#cos"]
pub extern "C" fn math_cos(_receiver: *const u8, x: SkFloat) -> SkFloat {
    x.val().cos().into()
}

#[export_name = "Meta:Math#sqrt"]
pub extern "C" fn math_sqrt(_receiver: *const u8, x: SkFloat) -> SkFloat {
    x.val().sqrt().into()
}
