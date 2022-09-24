/// An instance of `::Class`
mod witness_table;
use crate::builtin::class::witness_table::WitnessTable;
use crate::builtin::{SkAry, SkInt, SkStr};
use crate::sk_methods::meta_class_new;
use shiika_ffi_macro::shiika_method;
use std::collections::HashMap;
#[repr(C)]
#[derive(Debug)]
pub struct SkClass(*mut ShiikaClass);

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

    pub fn witness_table(&self) -> &WitnessTable {
        unsafe { (*self.0).witness_table.as_ref().unwrap() }
    }

    pub fn witness_table_mut(&mut self) -> &mut WitnessTable {
        unsafe { (*self.0).witness_table.as_mut().unwrap() }
    }

    fn erasure_class(&self) -> SkClass {
        let erasure_cls = unsafe { &(*self.0).erasure_cls };
        if erasure_cls.0.is_null() {
            self.dup()
        } else {
            erasure_cls.dup()
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaClass {
    vtable: *const u8,
    metacls_obj: SkClass,
    name: SkStr,
    specialized_classes: *mut HashMap<String, *mut ShiikaClass>,
    type_args: *mut Vec<SkClass>,
    witness_table: *mut WitnessTable,
    // `Array<Int>` -> `Array`
    // `Pair<Int, Bool>` -> `Pair`
    // `Object` -> null (means that its erasure is itself)
    erasure_cls: SkClass,
}

#[shiika_method("Meta:Class#_new")]
#[allow(non_snake_case)]
pub extern "C" fn meta_class__new(
    _receiver: *const u8,
    name: SkStr,
    vtable: *const u8,
    witness_table: *mut WitnessTable,
    metacls_obj: SkClass,
    erasure_cls: SkClass,
) -> SkClass {
    let cls_obj = meta_class_new(std::ptr::null());
    unsafe {
        (*cls_obj.0).vtable = vtable;
        (*cls_obj.0).name = name;
        (*cls_obj.0).metacls_obj = metacls_obj;
        (*cls_obj.0).erasure_cls = erasure_cls;
        (*cls_obj.0).specialized_classes = Box::leak(Box::new(HashMap::new()));
        if witness_table.is_null() {
            (*cls_obj.0).witness_table = Box::leak(Box::new(WitnessTable::new()));
        } else {
            (*cls_obj.0).witness_table = witness_table;
        }
    }
    cls_obj
}

#[shiika_method("Metaclass#_new")]
#[allow(non_snake_case)]
pub extern "C" fn metaclass__new(
    _receiver: *const u8,
    name: SkStr,
    vtable: *const u8,
    witness_table: *mut WitnessTable,
    metacls_obj: SkClass,
    erasure_cls: SkClass,
) -> SkClass {
    meta_class__new(
        _receiver,
        name,
        vtable,
        witness_table,
        metacls_obj,
        erasure_cls,
    )
}

// Returns the n-th type argument. Panics if the index is out of bound
#[shiika_method("Class#_type_argument")]
pub extern "C" fn class_type_argument(receiver: SkClass, nth: SkInt) -> SkClass {
    let v = unsafe { (*receiver.0).type_args.as_ref().unwrap() };
    v[nth.val() as usize].dup()
}

#[allow(non_snake_case)]
#[shiika_method("Class#<>")]
pub extern "C" fn class__specialize(receiver: SkClass, tyargs: SkAry<SkClass>) -> SkClass {
    class_specialize(receiver, tyargs.into_vec())
}

/// Same as `Class#<>` but does not need `Array` to call.
/// Used for solving bootstrap problem
#[allow(non_snake_case)]
#[shiika_method("Class#_specialize1")]
pub extern "C" fn class__specialize1(receiver: SkClass, tyarg: SkClass) -> SkClass {
    class_specialize(receiver, vec![tyarg])
}

/// Create a specialized class from a generic class
/// eg. make `Array<Int>` from `Array` and `Int`
fn class_specialize(mut receiver: SkClass, tyargs: Vec<SkClass>) -> SkClass {
    let name = specialized_name(&receiver, &tyargs);
    if let Some(c) = receiver.specialized_classes().get(&name) {
        SkClass::new(*c)
    } else {
        let spe_meta = if receiver.metacls_obj().name().as_str() == "Metaclass" {
            receiver.metacls_obj()
        } else {
            let cloned = tyargs.iter().map(SkClass::dup).collect();
            class_specialize(receiver.metacls_obj(), cloned)
        };
        let c = meta_class__new(
            std::ptr::null(),
            name.clone().into(),
            receiver.vtable(),
            receiver.witness_table_mut(),
            spe_meta,
            receiver.dup(),
        );
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
fn specialized_name(class: &SkClass, tyargs: &[SkClass]) -> String {
    let args = tyargs
        .iter()
        .map(|cls| cls.name().as_str().to_string())
        .collect::<Vec<_>>();
    format!("{}<{}>", class.name().as_str(), args.join(", "))
}

#[shiika_method("Class#erasure_class")]
pub extern "C" fn class_erasure_class(receiver: SkClass) -> SkClass {
    receiver.erasure_class()
}
