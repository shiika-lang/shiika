/// An instance of `::Class`
use crate::builtin::SkStr;
use shiika_ffi_macro::shiika_method;
use std::collections::HashMap;
#[repr(C)]
#[derive(Debug)]
pub struct SkClass(*const ShiikaClass);

//extern "C" {
//    fn box_int(i: i64) -> SkInt;
//}

impl SkClass {
    pub fn new(ptr: *const ShiikaClass) -> SkClass {
        SkClass(ptr)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaClass {
    vtable: *const u8,
    metaclass_obj: *const ShiikaClass,
    name: SkStr,
    specialized_classes: HashMap<String, SkClass>,
}

#[shiika_method("Class#_initialize_rustlib")]
#[allow(non_snake_case)]
pub extern "C" fn class__initialize_rustlib(
    receiver: *mut ShiikaClass,
    vtable: *const u8,
    metaclass_obj: *const ShiikaClass,
) -> &'static mut HashMap<String, SkClass> {
    unsafe {
        (*receiver).vtable = vtable;
        (*receiver).metaclass_obj = metaclass_obj;
    }
    let hash: HashMap<String, SkClass> = HashMap::new();
    let leaked = Box::leak(Box::new(hash));
    leaked
}

//#[shiika_method("Metaclass#_specialize")]
//pub extern "C" fn metaclass__specialize(receiver: SkCls, tyargs: SkAry) -> SkCls {
//    unsafe {}
//}
