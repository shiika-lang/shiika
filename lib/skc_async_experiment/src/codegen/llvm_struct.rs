use crate::codegen::CodeGen;
use inkwell::types::BasicType;

/// Number of elements before ivars
pub const OBJ_HEADER_SIZE: usize = 2;
/// 0th: reference to the vtable
pub const OBJ_VTABLE_IDX: usize = 0;
/// 1st: reference to the class object
pub const OBJ_CLASS_IDX: usize = 1;

pub fn define(gen: &mut CodeGen) {
    define_int(gen);
}

fn define_int(gen: &mut CodeGen) {
    let struct_type = gen.context.opaque_struct_type(&"::Int");
    let vt = gen.ptr_type().into(); //TODO: vtable
    let ct = gen.ptr_type().into(); //TODO: class type
    struct_type.set_body(&[vt, ct, gen.context.i64_type().into()], false);
}

pub fn get<'run>(gen: &CodeGen, name: &str) -> inkwell::types::StructType<'run> {
    gen.context
        .get_struct_type(name)
        .expect(&format!("struct type not found: {}", name))
}

pub fn build_llvm_value_store<'run>(
    gen: &mut CodeGen,
    struct_type: &inkwell::types::StructType<'run>,
    struct_ptr: inkwell::values::PointerValue<'run>,
    idx: usize,
    value: inkwell::values::BasicValueEnum,
    name: &str,
) {
    let ptr = gen
        .builder
        .build_struct_gep(
            struct_type.as_basic_type_enum(),
            struct_ptr,
            idx as u32,
            &format!("addr_{}", name),
        )
        .unwrap_or_else(|_| {
            panic!(
                "build_llvm_struct_set: elem not found (idx in struct: {}, register name: {}, struct: {:?})",
                &idx, &name, &struct_ptr
            )
        });
    gen.builder.build_store(ptr, value);
}
