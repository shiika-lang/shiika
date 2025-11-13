use crate::codegen::{
    llvm_struct,
    value::{SkClassObj, SkObj},
    vtable, CodeGen,
};
use crate::names::FunctionName;
use inkwell::values::{AnyValue, BasicValue, BasicValueEnum};
use shiika_core::names::class_fullname;

/// Number of elements before ivars
const OBJ_HEADER_SIZE: usize = 2;
/// 0th: reference to the vtable
const OBJ_VTABLE_IDX: usize = 0;
/// 1st: reference to the class object
const OBJ_CLASS_IDX: usize = 1;

pub fn build_ivar_load_raw<'run>(
    gen: &mut CodeGen<'run, '_>,
    sk_obj: SkObj<'run>,
    struct_type: inkwell::types::StructType<'run>,
    item_type: inkwell::types::BasicTypeEnum<'run>,
    idx: usize,
    name: &str,
) -> inkwell::values::BasicValueEnum<'run> {
    let i = OBJ_HEADER_SIZE + idx;
    let ptr = sk_obj.0;
    llvm_struct::build_llvm_value_load(gen, struct_type, ptr, item_type, i, name)
}

//pub fn build_ivar_store<'run>(
//    gen: &mut CodeGen,
//    sk_obj: SkObj,
//    struct_type: &inkwell::types::StructType<'run>,
//    idx: usize,
//    value: SkObj<'run>,
//    name: &str,
//) {
//    let llvm_value = value.0.as_basic_value_enum();
//    build_ivar_store_raw(gen, sk_obj, struct_type, idx, llvm_value, name);
//}

pub fn build_ivar_store_raw<'run>(
    gen: &mut CodeGen<'run, '_>,
    sk_obj: SkObj<'run>,
    struct_type: &inkwell::types::StructType<'run>,
    idx: usize,
    value: inkwell::values::BasicValueEnum,
    name: &str,
) {
    let i = OBJ_HEADER_SIZE + idx;
    let ptr = sk_obj.0;
    llvm_struct::build_llvm_value_store(gen, struct_type, ptr, i, value, name);
}

/// Get the vtable of a Shiika object.
pub fn get_vtable<'run>(
    gen: &mut CodeGen<'run, '_>,
    obj: &SkObj<'run>,
) -> vtable::OpaqueVTableRef<'run> {
    let s = llvm_struct::get(gen, "Object");
    let ty = gen.ptr_type();
    let v = llvm_struct::build_llvm_value_load(gen, s, obj.0, ty.into(), OBJ_VTABLE_IDX, "vtable");
    vtable::OpaqueVTableRef {
        ptr: v.into_pointer_value(),
    }
}

/// Set `vtable` to `object`
fn set_vtable<'run>(
    gen: &mut CodeGen<'run, '_>,
    object: &SkObj<'run>,
    vtable: vtable::OpaqueVTableRef<'run>,
) {
    let v = vtable.ptr.as_basic_value_enum();
    let s = llvm_struct::get(gen, "Object");
    llvm_struct::build_llvm_value_store(gen, &s, object.0, OBJ_VTABLE_IDX, v, "vtable");
}

/// Set `class_obj` to the class object field of `object`
pub fn set_class_obj<'run>(
    gen: &mut CodeGen<'run, '_>,
    object: &SkObj<'run>,
    class_obj: SkClassObj<'run>,
) {
    let s = llvm_struct::get(gen, "Object");
    llvm_struct::build_llvm_value_store(
        gen,
        &s,
        object.0,
        OBJ_CLASS_IDX,
        class_obj.0.as_basic_value_enum(),
        "my_class",
    );
}

pub fn allocate_sk_obj<'run>(gen: &mut CodeGen<'run, '_>, class_name: &str) -> SkObj<'run> {
    let class_name_ = class_fullname(class_name);
    let t = llvm_struct::get(gen, &class_name);
    let obj = SkObj(allocate_mem(gen, &t));

    let vtable = vtable::get(gen, &class_name_);
    set_vtable(gen, &obj, vtable);

    let class_obj = SkClassObj::load(gen, &class_name_);
    set_class_obj(gen, &obj, class_obj);

    obj
}

/// Allocate some memory for a value of LLVM type `t`. Returns void ptr.
fn allocate_mem<'run>(
    gen: &mut CodeGen<'run, '_>,
    t: &inkwell::types::StructType<'run>,
) -> inkwell::values::PointerValue<'run> {
    let size = t.size_of().expect("type has no size");
    shiika_malloc(gen, size)
}

/// Call `shiika_malloc`
fn shiika_malloc<'run>(
    gen: &mut CodeGen<'run, '_>,
    size: inkwell::values::IntValue<'run>,
) -> inkwell::values::PointerValue<'run> {
    let func = gen.get_llvm_func(&FunctionName::mangled("shiika_malloc"));
    let call_result = gen
        .builder
        .build_direct_call(func, &[size.as_basic_value_enum().into()], "mem")
        .unwrap();
    let basic_value: BasicValueEnum = call_result.as_any_value_enum().try_into().unwrap();
    basic_value.into_pointer_value()
}
