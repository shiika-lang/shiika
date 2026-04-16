use shiika_ffi::core_class::{SkBool, SkInt, SkString};
use shiika_ffi_macro::{async_shiika_method, shiika_method};

#[shiika_method("String#initialize")]
pub extern "C" fn string_initialize(mut receiver: SkString, bytes: *const u8, n_bytes: u64) {
    unsafe {
        let slice = std::slice::from_raw_parts(bytes, n_bytes as usize);
        receiver.set_value(slice.to_vec());
    }
}

#[shiika_method("String#+")]
pub extern "C" fn string_add(receiver: SkString, other: SkString) -> SkString {
    let mut result = receiver.value().to_vec();
    result.extend_from_slice(other.value());
    SkString::from_vec(result)
}

#[shiika_method("String#*")]
pub extern "C" fn string_mul(receiver: SkString, n: SkInt) -> SkString {
    let val = receiver.value();
    let count = n.val() as usize;
    let mut result = Vec::with_capacity(val.len() * count);
    for _ in 0..count {
        result.extend_from_slice(val);
    }
    SkString::from_vec(result)
}

#[shiika_method("String#==")]
pub extern "C" fn string_eq(receiver: SkString, other: SkString) -> SkBool {
    (receiver.value() == other.value()).into()
}

#[shiika_method("String#bytesize")]
pub extern "C" fn string_bytesize(receiver: SkString) -> SkInt {
    (receiver.value().len() as i64).into()
}

#[shiika_method("String#empty?")]
pub extern "C" fn string_is_empty(receiver: SkString) -> SkBool {
    receiver.value().is_empty().into()
}

#[shiika_method("String#starts_with?")]
pub extern "C" fn string_starts_with(receiver: SkString, s: SkString) -> SkBool {
    receiver.value().starts_with(s.value()).into()
}

#[shiika_method("String#ends_with?")]
pub extern "C" fn string_ends_with(receiver: SkString, s: SkString) -> SkBool {
    receiver.value().ends_with(s.value()).into()
}

#[shiika_method("String#nth_byte")]
pub extern "C" fn string_nth_byte(receiver: SkString, n: SkInt) -> SkInt {
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

#[shiika_method("String#slice_bytes")]
pub extern "C" fn string_slice_bytes(receiver: SkString, from: SkInt, bytes: SkInt) -> SkString {
    let from_val = from.val();
    let bytes_val = bytes.val();
    let val = receiver.value();
    if from_val < 0 {
        panic!(
            "[String#slice_bytes: `from` is less than zero (from: {}, bytes: {})]",
            from_val, bytes_val
        );
    }
    let end = from_val as usize + bytes_val as usize;
    if end > val.len() {
        panic!(
            "[String#slice_bytes: `from + bytes` too large (from: {}, bytes: {}, self.bytesize: {})]",
            from_val, bytes_val, val.len()
        );
    }
    let slice = &val[from_val as usize..end];
    SkString::from_vec(slice.to_vec())
}

#[shiika_method("String#to_i")]
pub extern "C" fn string_to_i(receiver: SkString) -> SkInt {
    let val = receiver.value();
    if val.is_empty() {
        return (0i64).into();
    }
    let mut start = 0;
    let mut minus = false;
    if val[0] == b'+' {
        start = 1;
    } else if val[0] == b'-' {
        start = 1;
        minus = true;
    }
    let mut n: i64 = 0;
    for &b in &val[start..] {
        if b >= b'0' && b <= b'9' {
            n = n * 10 + (b - b'0') as i64;
        } else {
            break;
        }
    }
    if minus {
        (-n).into()
    } else {
        n.into()
    }
}

#[async_shiika_method("String#to_s")]
async fn string_to_s(receiver: SkString) -> SkString {
    receiver
}

#[async_shiika_method("String#inspect")]
async fn string_inspect(receiver: SkString) -> SkString {
    let mut result = Vec::new();
    result.push(b'"');
    result.extend_from_slice(receiver.value());
    result.push(b'"');
    SkString::from_vec(result)
}

#[shiika_method("String#ljust")]
pub extern "C" fn string_ljust(receiver: SkString, width: SkInt, padding: SkString) -> SkString {
    let width_val = width.val() as usize;
    let self_val = receiver.value();
    let pad_val = padding.value();
    if self_val.len() >= width_val || pad_val.is_empty() {
        return SkString::from_vec(self_val.to_vec());
    }
    let mut result = self_val.to_vec();
    while result.len() < width_val {
        result.extend_from_slice(pad_val);
    }
    SkString::from_vec(result)
}

#[shiika_method("String#rjust")]
pub extern "C" fn string_rjust(receiver: SkString, width: SkInt, padding: SkString) -> SkString {
    let width_val = width.val() as usize;
    let self_val = receiver.value();
    let pad_val = padding.value();
    if self_val.len() >= width_val || pad_val.is_empty() {
        return SkString::from_vec(self_val.to_vec());
    }
    let mut prefix = Vec::new();
    while prefix.len() + self_val.len() < width_val {
        prefix.extend_from_slice(pad_val);
    }
    prefix.extend_from_slice(self_val);
    SkString::from_vec(prefix)
}
