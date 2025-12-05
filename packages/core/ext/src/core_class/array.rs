use shiika_ffi::core_class::{SkArray, SkInt, SkObject};
use shiika_ffi_macro::shiika_method;

/// Creates a new Array instance from raw array data and length
/// Called from LLVM-generated code for array literals
#[shiika_method("Array#initialize")]
pub extern "C" fn array_initialize(
    receiver: SkArray<SkObject>,
    raw_array_ptr: *const SkObject,
    length: u64,
) {
    let len = length as usize;
    let mut vec = Vec::with_capacity(len);

    // Copy elements from the raw array into the Vec
    for i in 0..len {
        unsafe {
            let elem_ptr = raw_array_ptr.add(i);
            let elem = std::ptr::read(elem_ptr);
            vec.push(elem);
        }
    }

    receiver.set_vec(vec);
}

#[shiika_method("Array#[]")]
pub extern "C" fn array_get(receiver: SkArray<SkObject>, idx: SkInt) -> SkObject {
    let v: &Vec<SkObject> = receiver.as_vec();
    v.get(idx.val() as usize)
        .unwrap_or_else(|| panic!("Array#[]: idx too large (len: {}, idx: {})", v.len(), idx))
        .dup()
}

#[shiika_method("Array#[]=")]
pub extern "C" fn array_set(receiver: SkArray<SkObject>, idx: SkInt, obj: SkObject) {
    let v = receiver.as_vec_mut();
    v[idx.val() as usize] = obj;
}

#[shiika_method("Array#clear")]
pub extern "C" fn array_clear(receiver: SkArray<SkObject>) {
    receiver.as_vec_mut().clear();
}

#[shiika_method("Array#length")]
pub extern "C" fn array_length(receiver: SkArray<SkObject>) -> SkInt {
    let l = receiver.as_vec().len();
    (l as i64).into()
}

#[shiika_method("Array#push")]
pub extern "C" fn array_push(receiver: SkArray<SkObject>, item: SkObject) {
    receiver.as_vec_mut().push(item);
}

#[shiika_method("Array#pop")]
pub extern "C" fn array_pop(receiver: SkArray<SkObject>) -> SkObject {
    receiver.as_vec_mut().pop().unwrap()
}

#[shiika_method("Array#reserve")]
pub extern "C" fn array_reserve(receiver: SkArray<SkObject>, additional: SkInt) {
    receiver.as_vec_mut().reserve(additional.val() as usize);
}

#[shiika_method("Array#shift")]
pub extern "C" fn array_shift(receiver: SkArray<SkObject>) -> SkObject {
    receiver.as_vec_mut().remove(0)
}
