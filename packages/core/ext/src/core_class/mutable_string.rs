use shiika_ffi::core_class::{SkBool, SkInt, SkMutableString, SkString};
use shiika_ffi_macro::shiika_method;

#[shiika_method("MutableString#initialize")]
pub extern "C" fn mutable_string_initialize(receiver: SkMutableString) {
    receiver.set_value(Vec::new());
}

#[shiika_method("MutableString#nth_byte")]
pub extern "C" fn mutable_string_nth_byte(receiver: SkMutableString, n: SkInt) -> SkInt {
    let idx = n.val();
    let val = receiver.value();
    if idx < 0 {
        panic!("[String#nth_byte: index less than zero]");
    }
    if idx as usize >= val.len() {
        panic!("[String#nth_byte: index too large]");
    }
    (val[idx as usize] as i64).into()
}

#[shiika_method("MutableString#append")]
pub extern "C" fn mutable_string_append(receiver: SkMutableString, other: SkString) {
    receiver.value_mut().extend_from_slice(other.value());
}

#[shiika_method("MutableString#append_byte")]
pub extern "C" fn mutable_string_append_byte(receiver: SkMutableString, b: SkInt) {
    receiver.value_mut().push(b.val() as u8);
}

#[shiika_method("MutableString#empty?")]
pub extern "C" fn mutable_string_is_empty(receiver: SkMutableString) -> SkBool {
    receiver.value().is_empty().into()
}

#[shiika_method("MutableString#to_s")]
pub extern "C" fn mutable_string_to_s(receiver: SkMutableString) -> SkString {
    SkString::from_vec(receiver.value().to_vec())
}

#[shiika_method("MutableString#write_byte")]
pub extern "C" fn mutable_string_write_byte(receiver: SkMutableString, nth: SkInt, byte: SkInt) {
    let n = nth.val();
    let b = byte.val();
    if n < 0 {
        panic!("[String#write_byte: index less than zero]");
    }
    if b < 0 {
        panic!("[String#write_byte: byte less than zero]");
    }
    if b >= 256 {
        panic!("[String#write_byte: byte larger than 255]");
    }
    let vec = receiver.value_mut();
    let idx = n as usize;
    if idx >= vec.len() {
        vec.resize(idx + 1, 0);
    }
    vec[idx] = b as u8;
}

#[shiika_method("MutableString#_unsafe_to_s")]
pub extern "C" fn mutable_string_unsafe_to_s(receiver: SkMutableString) -> SkString {
    SkString::from_vec(receiver.value().to_vec())
}
