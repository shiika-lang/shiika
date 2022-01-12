/// An instance of `::Class`
use crate::builtin::{SkAry, SkStr};
use shiika_ffi_macro::shiika_method;
use std::collections::HashMap;
#[repr(C)]
#[derive(Debug)]
pub struct SkClass(*mut ShiikaClass);

extern "C" {
    // SkClass contains *mut of `HashMap`, which is not `repr(C)`.
    // I think it's ok because the hashmap is not accessible in Shiika.
    // TODO: is there a better way?
    #[allow(improper_ctypes)]
    fn Meta_SpecializedClass_new(
        receiver: *const u8,
        name: SkStr,
        vtable: *const u8,
        metacls_obj: SkClass,
        type_args: SkAry<ShiikaClass>,
    ) -> SkClass;
}

impl SkClass {
    pub fn new(ptr: *mut ShiikaClass) -> SkClass {
        SkClass(ptr)
    }

    pub fn dup(&self) -> SkClass {
        SkClass(self.0)
    }

    fn vtable(&self) -> *const u8 {
        unsafe { (*self.0).vtable }
    }

    fn metacls_obj(&self) -> SkClass {
        let metacls_obj = unsafe { &(*self.0).metacls_obj };
        SkClass::new(metacls_obj.0)
    }

    fn name(&self) -> &SkStr {
        unsafe { &(*self.0).name }
    }

    fn specialized_classes(&mut self) -> &mut HashMap<String, *mut ShiikaClass> {
        unsafe { (*self.0).specialized_classes.as_mut().unwrap() }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaClass {
    vtable: *const u8,
    metacls_obj: SkClass,
    name: SkStr,
    specialized_classes: *mut HashMap<String, *mut ShiikaClass>,
}

#[shiika_method("Class#_initialize_rustlib")]
#[allow(non_snake_case)]
pub extern "C" fn class__initialize_rustlib(
    receiver: *mut ShiikaClass,
    vtable: *const u8,
    metacls_obj: SkClass,
) -> &'static mut HashMap<String, SkClass> {
    let leaked = Box::leak(Box::new(HashMap::new()));
    unsafe {
        (*receiver).vtable = vtable;
        (*receiver).metacls_obj = metacls_obj;
        //(*receiver).specialized_classes = leaked;
    }
    // REFACTOR: Returning the hashmap because there is no other way to
    // tell Shiika that `Class` has ivar `@specialized_classes` with type
    // `Object` (see also builtin/class.sk)
    leaked
}

/// Create a specialized class from a generic class
/// eg. make `Array<Int>` from `Array` and `Int`
#[allow(non_snake_case)]
#[shiika_method("Class#_specialize")]
pub extern "C" fn class__specialize(mut receiver: SkClass, tyargs: SkAry<ShiikaClass>) -> SkClass {
    let name = specialized_name(&receiver, &tyargs);
    if let Some(c) = receiver.specialized_classes().get(&name) {
        SkClass::new(*c)
    } else {
        let spe_meta = if receiver.metacls_obj().name().as_str() == "Metaclass" {
            receiver.metacls_obj()
        } else {
            class__specialize(receiver.metacls_obj(), tyargs.dup())
        };
        let c = unsafe {
            Meta_SpecializedClass_new(
                std::ptr::null(),
                name.clone().into(),
                receiver.vtable(),
                spe_meta,
                tyargs,
            )
        };
        receiver.specialized_classes().insert(name, c.0);
        c
    }
}

/// Returns a string like `"Array<Int>"`
fn specialized_name(class: &SkClass, tyargs: &SkAry<ShiikaClass>) -> String {
    let args = tyargs
        .iter()
        .map(|obj| SkClass::new(obj).name().as_str().to_string())
        .collect::<Vec<_>>();
    format!("{}<{}>", class.name().as_str(), args.join(", "))
}
