/// An instance of `::Class`
use crate::builtin::{SkAry, SkInt, SkStr};
use shiika_ffi_macro::shiika_method;
use std::collections::HashMap;
#[repr(C)]
#[derive(Debug)]
pub struct SkModule(*mut ShiikaClass);

extern "C" {
    // SkModule contains *mut of `HashMap`, which is not `repr(C)`.
    // I think it's ok because the hashmap is not accessible in Shiika.
    // TODO: is there a better way?
    // TODO: macro to convert "Meta:Class#new" into this name
    #[allow(improper_ctypes)]
    fn Meta_Class_new(
        receiver: *const u8,
        name: SkStr,
        vtable: *const u8,
        metacls_obj: SkModule,
    ) -> SkModule;
}

impl SkModule {
    pub fn new(ptr: *mut ShiikaClass) -> SkModule {
        SkModule(ptr)
    }

    pub fn dup(&self) -> SkModule {
        SkModule(self.0)
    }

    fn vtable(&self) -> *const u8 {
        unsafe { (*self.0).vtable }
    }

    fn metacls_obj(&self) -> SkModule {
        let metacls_obj = unsafe { &(*self.0).metacls_obj };
        SkModule::new(metacls_obj.0)
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
    metacls_obj: SkModule,
    name: SkStr,
    specialized_classes: *mut HashMap<String, *mut ShiikaClass>,
    type_args: *mut Vec<SkModule>,
}

#[shiika_method("Class#_initialize_rustlib")]
#[allow(non_snake_case)]
pub extern "C" fn class__initialize_rustlib(
    receiver: *mut ShiikaClass,
    vtable: *const u8,
    metacls_obj: SkModule,
) {
    unsafe {
        (*receiver).vtable = vtable;
        (*receiver).metacls_obj = metacls_obj;
        (*receiver).specialized_classes = Box::leak(Box::new(HashMap::new()));
    }
}

// Returns the n-th type argument. Panics if the index is out of bound
#[shiika_method("Class#_type_argument")]
pub extern "C" fn class_type_argument(receiver: SkModule, nth: SkInt) -> SkModule {
    let v = unsafe { (*receiver.0).type_args.as_ref().unwrap() };
    v[nth.val() as usize].dup()
}

#[allow(non_snake_case)]
#[shiika_method("Class#<>")]
pub extern "C" fn class__specialize(receiver: SkModule, tyargs_: SkAry<ShiikaClass>) -> SkModule {
    let tyargs = tyargs_.iter().map(|ptr| SkModule::new(ptr)).collect();
    class_specialize(receiver, tyargs)
}

/// Same as `Class#<>` but does not need `Array` to call.
/// Used for solving bootstrap problem
#[allow(non_snake_case)]
#[shiika_method("Class#_specialize1")]
pub extern "C" fn class__specialize1(receiver: SkModule, tyarg: SkModule) -> SkModule {
    class_specialize(receiver, vec![tyarg])
}

/// Create a specialized class from a generic class
/// eg. make `Array<Int>` from `Array` and `Int`
fn class_specialize(mut receiver: SkModule, tyargs: Vec<SkModule>) -> SkModule {
    let name = specialized_name(&receiver, &tyargs);
    if let Some(c) = receiver.specialized_classes().get(&name) {
        SkModule::new(*c)
    } else {
        let spe_meta = if receiver.metacls_obj().name().as_str() == "Metaclass" {
            receiver.metacls_obj()
        } else {
            let cloned = tyargs.iter().map(SkModule::dup).collect();
            class_specialize(receiver.metacls_obj(), cloned)
        };
        let c = unsafe {
            Meta_Class_new(
                std::ptr::null(),
                name.clone().into(),
                receiver.vtable(),
                spe_meta,
            )
        };
        unsafe {
            // Q. Why not just `(*c.0).type_args = tyargs` ?
            // A. To avoid `improper_ctypes` warning of some extern funcs.
            (*c.0).type_args = Box::into_raw(Box::new(tyargs));
        }
        receiver.specialized_classes().insert(name, c.0);
        c
    }
}

/// Returns a string like `"Array<Int>"`
fn specialized_name(class: &SkModule, tyargs: &[SkModule]) -> String {
    let args = tyargs
        .iter()
        .map(|cls| cls.name().as_str().to_string())
        .collect::<Vec<_>>();
    format!("{}<{}>", class.name().as_str(), args.join(", "))
}
