use shiika_ffi::core_class::SkString;
use shiika_ffi_macro::shiika_method;

#[shiika_method("String#initialize")]
pub extern "C" fn string_initialize(mut receiver: SkString, bytes: *const u8, n_bytes: u64) {
    unsafe {
        let slice = std::slice::from_raw_parts(bytes, n_bytes as usize);
        receiver.set_value(slice);
    }
}
