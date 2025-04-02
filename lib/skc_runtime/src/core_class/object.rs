use shiika_ffi::core_class::SkObject;
use shiika_ffi_macro::shiika_method;

#[shiika_method("Object#initialize")]
pub extern "C" fn object_initialize(_receiver: SkObject) {}
