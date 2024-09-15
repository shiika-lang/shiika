use crate::codegen::{llvm_struct, CodeGen, SkValue};
use inkwell::values::BasicValue;

pub fn build_ivar_store<'run>(
    gen: &mut CodeGen,
    sk_value: SkValue,
    struct_type: &inkwell::types::StructType<'run>,
    idx: usize,
    value: SkValue,
    name: &str,
) {
    let llvm_value = value.0.as_basic_value_enum();
    build_ivar_store_raw(gen, sk_value, struct_type, idx, llvm_value, name);
}

pub fn build_ivar_store_raw<'run>(
    gen: &mut CodeGen,
    sk_value: SkValue,
    struct_type: &inkwell::types::StructType<'run>,
    idx: usize,
    value: inkwell::values::BasicValueEnum,
    name: &str,
) {
    let i = llvm_struct::OBJ_HEADER_SIZE + idx;
    let ptr = sk_value.0;
    llvm_struct::build_llvm_value_store(gen, struct_type, ptr, i, value, name);
}

pub fn allocate_sk_obj<'run>(gen: &mut CodeGen<'run, '_>, name: &str) -> SkValue<'run> {
    let t = gen
        .context
        .get_struct_type(name)
        .expect(&format!("struct type not found: {}", name));
    SkValue(allocate_mem(gen, &t))
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
    let func = gen.get_llvm_func("shiika_malloc");
    gen.builder
        .build_direct_call(func, &[size.as_basic_value_enum().into()], "mem")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}
