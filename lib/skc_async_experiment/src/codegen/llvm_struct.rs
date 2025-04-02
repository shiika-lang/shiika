use crate::codegen::CodeGen;
use crate::mir::MirClass;
use inkwell::types::BasicType;

/// Number of elements before ivars
pub const OBJ_HEADER_SIZE: usize = 2;
/// 0th: reference to the vtable
//const OBJ_VTABLE_IDX: usize = 0;
/// 1st: reference to the class object
//const OBJ_CLASS_IDX: usize = 1;

pub fn define(gen: &mut CodeGen, classes: &[MirClass]) {
    for class in classes {
        define_class_struct(gen, class);
    }
    //    define_bool(gen);
    //    define_int(gen);
    //    define_void(gen);
}

fn define_class_struct(gen: &mut CodeGen, class: &MirClass) {
    dbg!(&class.name);
    let struct_type = gen.context.opaque_struct_type(&class.name);
    let vtable = gen.ptr_type().into();
    let class_obj = gen.ptr_type().into();
    let ivars: Vec<inkwell::types::BasicTypeEnum> = match &class.name[..] {
        "Bool" => vec![gen.context.bool_type().into()],
        "Int" => vec![gen.context.i64_type().into()],
        _ => class
            .ivars
            .iter()
            .map(|(_, ty)| gen.llvm_type(ty).into())
            .collect::<Vec<_>>(),
    };
    let mut elems = vec![vtable, class_obj];
    elems.extend(ivars);
    struct_type.set_body(&elems, false);
}

//fn define_bool(gen: &mut CodeGen) {
//    let struct_type = gen.context.opaque_struct_type(&"::Bool");
//    let vtable = gen.ptr_type().into();
//    let class = gen.ptr_type().into();
//    let value = gen.context.bool_type().into();
//    struct_type.set_body(&[vtable, class, value], false);
//}
//
//fn define_int(gen: &mut CodeGen) {
//    let struct_type = gen.context.opaque_struct_type(&"::Int");
//    let vtable = gen.ptr_type().into();
//    let class = gen.ptr_type().into();
//    let value = gen.context.i64_type().into();
//    struct_type.set_body(&[vtable, class, value], false);
//}
//
//fn define_void(gen: &mut CodeGen) {
//    let struct_type = gen.context.opaque_struct_type(&"::Void");
//    let vtable = gen.ptr_type().into();
//    let class = gen.ptr_type().into();
//    struct_type.set_body(&[vtable, class], false);
//}

pub fn get<'run>(gen: &CodeGen, name: &str) -> inkwell::types::StructType<'run> {
    gen.context
        .get_struct_type(name)
        .expect(&format!("struct type not found: {}", name))
}

pub fn build_llvm_value_load<'run>(
    gen: &mut CodeGen<'run, '_>,
    struct_type: inkwell::types::StructType<'run>,
    struct_ptr: inkwell::values::PointerValue<'run>,
    item_type: inkwell::types::BasicTypeEnum<'run>,
    idx: usize,
    name: &str,
) -> inkwell::values::BasicValueEnum<'run> {
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
                "build_llvm_value_load: elem not found (idx in struct: {}, register name: {}, struct: {:?})",
                &idx, &name, &struct_ptr
            )
        });
    gen.builder
        .build_load(item_type, ptr, &format!("load_{}", name))
}

pub fn build_llvm_value_store<'run>(
    gen: &mut CodeGen<'run, '_>,
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
