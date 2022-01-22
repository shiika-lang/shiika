use crate::builtin::class::SkClass;
use crate::builtin::{SkInt, SkStr};
use shiika_ffi_macro::shiika_method;

#[repr(C)]
#[derive(Debug)]
pub struct SkSpecializedClass(*mut ShiikaSpecializedClass);

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaSpecializedClass {
    vtable: *const u8,
    metacls_obj: SkClass,
    name: SkStr,
    type_args: *const Vec<SkClass>,
}

#[shiika_method("Class::SpecializedClass#_type_argument")]
pub extern "C" fn specialized_class_type_argument(
    receiver: SkSpecializedClass,
    nth: SkInt,
) -> SkClass {
    let v = unsafe { (*receiver.0).type_args.as_ref().unwrap() };
    v[nth.val() as usize].dup()
}
