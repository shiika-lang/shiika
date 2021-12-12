use crate::builtin::object::ShiikaObject;
use crate::builtin::{SkInt, SkPtr};
use shiika_ffi_macro::shiika_method;

#[repr(C)]
#[derive(Debug)]
pub struct SkAry(*mut ShiikaArray);

#[repr(C)]
#[derive(Debug)]
struct ShiikaArray {
    vtable: *const u8,
    class_obj: *const u8,
    capa: SkInt,
    n_items: SkInt,
    items: SkPtr,
}

#[shiika_method("Array#[]")]
pub extern "C" fn array_get(receiver: SkAry, idx: SkInt) -> *const ShiikaObject {
    unsafe {
        let items_ptr = (*receiver.0).items.unbox() as *const *const ShiikaObject;
        let item_ptr = items_ptr.offset(idx.val() as isize);
        *item_ptr
    }
}
