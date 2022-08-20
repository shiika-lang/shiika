use crate::builtin::{SkInt, SkObj};
use shiika_ffi_macro::shiika_method;

extern "C" {
    /// `Array.new`
    /// `receiver` should be the class `Array` but currently may be just `null`.
    #[allow(improper_ctypes)]
    fn Meta_Array_new(receiver: *const u8) -> SkAry<SkObj>;
}

#[repr(C)]
#[derive(Debug)]
pub struct SkAry<T>(*mut ShiikaArray<T>);

#[repr(C)]
#[derive(Debug)]
struct ShiikaArray<T> {
    vtable: *const u8,
    class_obj: *const u8,
    vec: *mut Vec<T>,
}

impl<T> SkAry<T> {
    /// Call `Array.new`.
    pub fn new<U>() -> SkAry<U> {
        unsafe {
            let sk_ary = Meta_Array_new(std::ptr::null());
            // Force cast because external function (Meta_Array_new)
            // cannot have type a parameter.
            SkAry(sk_ary.0 as *mut ShiikaArray<U>)
        }
    }

    pub fn as_vec(&self) -> &Vec<T> {
        unsafe { (*self.0).vec.as_ref().unwrap() }
    }

    pub fn as_vec_mut(&self) -> &mut Vec<T> {
        unsafe { (*self.0).vec.as_mut().unwrap() }
    }

    pub fn into_vec(&self) -> Vec<T> {
        unsafe { (*self.0).vec.read() }
    }

    /// Replace the contents with `v`.
    /// The original Vec will be free'd by GC.
    pub fn set_vec(&self, v: Vec<T>) {
        unsafe { (*self.0).vec = Box::leak(Box::new(v)) }
    }
}

/// Called from `Array.new` and initializes internal fields.
#[shiika_method("Array#_initialize_rustlib")]
#[allow(non_snake_case)]
pub extern "C" fn array__initialize_rustlib(receiver: SkAry<SkObj>) {
    unsafe {
        (*receiver.0).vec = Box::leak(Box::new(Vec::new()));
    }
}

#[shiika_method("Array#[]")]
pub extern "C" fn array_get(receiver: SkAry<SkObj>, idx: SkInt) -> SkObj {
    let v: &Vec<SkObj> = receiver.as_vec();
    v.get(idx.val() as usize)
        .unwrap_or_else(|| panic!("Array#[]: idx too large (len: {}, idx: {})", v.len(), idx))
        .dup()
}

#[shiika_method("Array#[]=")]
pub extern "C" fn array_set(receiver: SkAry<SkObj>, idx: SkInt, obj: SkObj) {
    let v = receiver.as_vec_mut();
    v[idx.val() as usize] = obj;
}

#[shiika_method("Array#clear")]
pub extern "C" fn array_clear(receiver: SkAry<SkObj>) {
    receiver.as_vec_mut().clear();
}

#[shiika_method("Array#length")]
pub extern "C" fn array_length(receiver: SkAry<SkObj>) -> SkInt {
    receiver.as_vec().len().into()
}

#[shiika_method("Array#push")]
pub extern "C" fn array_push(receiver: SkAry<SkObj>, item: SkObj) {
    receiver.as_vec_mut().push(item);
}

#[shiika_method("Array#pop")]
pub extern "C" fn array_pop(receiver: SkAry<SkObj>) -> SkObj {
    receiver.as_vec_mut().pop().unwrap().dup()
}

#[shiika_method("Array#reserve")]
pub extern "C" fn array_reserve(receiver: SkAry<SkObj>, additional: SkInt) {
    receiver.as_vec_mut().reserve(additional.into());
}

#[shiika_method("Array#shift")]
pub extern "C" fn array_shift(receiver: SkAry<SkObj>) -> SkObj {
    receiver.as_vec_mut().remove(0)
}
