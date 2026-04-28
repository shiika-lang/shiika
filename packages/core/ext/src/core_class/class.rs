use shiika_ffi::core_class::class::{ShiikaClass, WitnessTable};
use shiika_ffi::core_class::{SkArray, SkBool, SkClass, SkInt, SkString};
use shiika_ffi_macro::{async_shiika_method, shiika_method};
use std::collections::HashMap;

#[async_shiika_method("Class#==")]
async fn class_eq(receiver: SkClass, other: SkClass) -> SkBool {
    (receiver.0 == other.0).into()
}

#[shiika_method("Meta:Class#_new")]
#[allow(non_snake_case)]
pub extern "C" fn meta_class__new(
    _receiver: *const u8,
    name: SkString,
    vtable: *const u8,
    witness_table: *mut WitnessTable,
    metaclass_obj: SkClass,
    erasure_cls: SkClass,
) -> SkClass {
    let wt = if witness_table.is_null() {
        Box::into_raw(Box::new(WitnessTable::new()))
    } else {
        witness_table
    };
    let shiika_class = ShiikaClass {
        vtable,
        metaclass_obj,
        name,
        specialized_classes: Box::into_raw(Box::new(HashMap::new())),
        type_args: std::ptr::null_mut(),
        witness_table: wt,
        erasure_cls,
    };
    SkClass::new(Box::into_raw(Box::new(shiika_class)))
}

#[shiika_method("Metaclass#_new")]
#[allow(non_snake_case)]
pub extern "C" fn metaclass__new(
    _receiver: *const u8,
    name: SkString,
    vtable: *const u8,
    witness_table: *mut WitnessTable,
    metaclass_obj: SkClass,
    erasure_cls: SkClass,
) -> SkClass {
    meta_class__new(
        _receiver,
        name,
        vtable,
        witness_table,
        metaclass_obj,
        erasure_cls,
    )
}

// Returns the n-th type argument. Panics if the index is out of bound
#[async_shiika_method("Class#_type_argument")]
async fn class_type_argument(receiver: SkClass, nth: SkInt) -> SkClass {
    let v = receiver.type_args();
    v[nth.val() as usize].dup()
}

#[async_shiika_method("Class#<>")]
async fn class_specialize_sym(receiver: SkClass, tyargs: SkArray<SkClass>) -> SkClass {
    class_specialize(receiver, tyargs.into_vec())
}

/// Same as `Class#<>` but does not need `Array` to call.
/// Used for solving bootstrap problem
#[async_shiika_method("Class#_specialize1")]
async fn class_specialize1(receiver: SkClass, tyarg: SkClass) -> SkClass {
    class_specialize(receiver, vec![tyarg])
}

/// Create a specialized class from a generic class
/// eg. make `Array<Int>` from `Array` and `Int`
fn class_specialize(mut receiver: SkClass, tyargs: Vec<SkClass>) -> SkClass {
    let name = specialized_name(&receiver, &tyargs);
    if let Some(c) = receiver.specialized_classes().get(&name) {
        SkClass::new(*c)
    } else {
        let metaclass_obj = receiver.metaclass_obj();
        let spe_meta = if metaclass_obj.0.is_null() || metaclass_obj.name().as_str() == "Metaclass"
        {
            metaclass_obj
        } else {
            let cloned = tyargs.iter().map(SkClass::dup).collect();
            class_specialize(metaclass_obj, cloned)
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

#[async_shiika_method("Class#erasure_class")]
async fn class_erasure_class(receiver: SkClass) -> SkClass {
    receiver.erasure_class()
}
