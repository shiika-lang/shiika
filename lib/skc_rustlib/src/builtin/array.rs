use crate::builtin::{SkInt, SkObj};

#[repr(C)]
#[derive(Debug)]
pub struct SkAry(*mut ShiikaArray);

#[repr(C)]
#[derive(Debug)]
struct ShiikaArray {
    vtable: *const u8,
    class_obj: *const u8,
    vec: *mut Vec<SkObj>,
}

impl SkAry {
    fn vec(&self) -> &mut Vec<SkObj> {
        unsafe {
            let vec_ptr = (*self.0).vec;
            &mut *vec_ptr
        }
    }
}

#[export_name = "Array#initialize"]
pub extern "C" fn array_initialize(receiver: SkAry) {
    let v = Box::new(vec![]);
    unsafe {
        (*receiver.0).vec = Box::leak(v);
    }
}

#[export_name = "Array#[]"]
pub extern "C" fn array_get(receiver: SkAry, idx: SkInt) -> SkObj {
    receiver.vec()[idx.val() as usize].dup_ptr()
}

#[export_name = "Array#push"]
pub extern "C" fn array_push(receiver: SkAry, item: SkObj) {
    receiver.vec().push(item)
}
