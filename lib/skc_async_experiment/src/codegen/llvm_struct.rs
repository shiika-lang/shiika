use crate::codegen::CodeGen;
use crate::mir;
use crate::mir::MirClass;
use anyhow::Result;
use inkwell::types::BasicType;
use shiika_core::ty::Erasure;

pub fn define(gen: &mut CodeGen, classes: &[MirClass]) {
    for class in classes {
        define_class_struct(gen, class);
    }
}

fn define_class_struct(gen: &mut CodeGen, class: &MirClass) {
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

/// Get the LLVM struct type for a given mir::Ty::Sk
/// Panics if not Ty::Sk
pub fn of_ty<'run>(gen: &CodeGen, ty: &mir::Ty) -> inkwell::types::StructType<'run> {
    let mir::Ty::Sk(term_ty) = ty else {
        panic!("expected mir::Ty::Sk, got {:?}", ty);
    };
    get(gen, &term_ty.erasure())
}

pub fn get<'run>(gen: &CodeGen, name: &Erasure) -> inkwell::types::StructType<'run> {
    gen.context
        .get_struct_type(&llvm_struct_name(name))
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
                "build_llvm_value_load: elem not found (idx in struct: {}, register name: {}, struct_type: {:?}, struct: {:?})",
                &idx, &name, &struct_type, &struct_ptr
            )
        });
    gen.builder
        .build_load(item_type, ptr, &format!("load_{}", name))
        .unwrap()
}

pub fn build_llvm_value_store<'run>(
    gen: &mut CodeGen<'run, '_>,
    struct_type: &inkwell::types::StructType<'run>,
    struct_ptr: inkwell::values::PointerValue<'run>,
    idx: usize,
    value: inkwell::values::BasicValueEnum,
    name: &str,
) -> Result<()> {
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
                "build_llvm_struct_set: elem not found (idx in struct: {}, register name: {}, struct_type: {:?}, struct: {:?})",
                &idx, &name, &struct_type, &struct_ptr
            )
        });
    gen.builder.build_store(ptr, value)?;
    Ok(())
}

fn llvm_struct_name(erasure: &Erasure) -> String {
    erasure.to_string()
}
