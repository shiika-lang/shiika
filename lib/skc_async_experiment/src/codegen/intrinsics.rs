//! Intrinsics are functions defined directly by the compiler.
use crate::codegen::{instance, llvm_struct, value::SkObj, CodeGen};
use anyhow::Result;
use inkwell::values::BasicValue;
use shiika_core::ty::Erasure;

pub fn define(gen: &mut CodeGen) -> Result<()> {
    define_box_int(gen)?;
    define_box_bool(gen)?;
    Ok(())
}

fn define_box_int(gen: &mut CodeGen) -> Result<()> {
    let fn_type = gen
        .ptr_type()
        .fn_type(&[gen.context.i64_type().into()], false);
    let function = gen
        .module
        .add_function("shiika_intrinsic_box_int", fn_type, None);
    let basic_block = gen.context.append_basic_block(function, "");
    gen.builder.position_at_end(basic_block);

    let i64_val = function.get_params()[0];
    let sk_int = instance::allocate_sk_obj(gen, &Erasure::nonmeta("Int"))?;
    let struct_type = llvm_struct::get(gen, &Erasure::nonmeta("Int"));
    instance::build_ivar_store_raw(gen, sk_int.clone(), &struct_type, 0, i64_val, "llvm_int")?;
    gen.builder.build_return(Some(&sk_int.0))?;
    Ok(())
}

fn define_box_bool(gen: &mut CodeGen) -> Result<()> {
    let fn_type = gen
        .ptr_type()
        .fn_type(&[gen.context.bool_type().into()], false);
    let function = gen
        .module
        .add_function("shiika_intrinsic_box_bool", fn_type, None);
    let basic_block = gen.context.append_basic_block(function, "");
    gen.builder.position_at_end(basic_block);

    let bool_val = function.get_params()[0];
    let sk_bool = instance::allocate_sk_obj(gen, &Erasure::nonmeta("Bool"))?;
    let struct_type = llvm_struct::get(gen, &Erasure::nonmeta("Bool"));
    instance::build_ivar_store_raw(gen, sk_bool.clone(), &struct_type, 0, bool_val, "llvm_bool")?;
    gen.builder.build_return(Some(&sk_bool.0))?;
    Ok(())
}

pub fn box_int<'run>(gen: &mut CodeGen<'run, '_>, n: i64) -> SkObj<'run> {
    let llvm_n = gen.context.i64_type().const_int(n as u64, false);
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_int",
            &[llvm_n.as_basic_value_enum().into()],
            "sk_int",
        )
        .unwrap()
        .into_pointer_value(),
    )
}

pub fn unbox_int<'run>(
    gen: &mut CodeGen<'run, '_>,
    sk_obj: SkObj<'run>,
) -> inkwell::values::IntValue<'run> {
    let struct_type = llvm_struct::get(gen, &Erasure::nonmeta("Int"));
    let item_type = gen.context.i64_type().into();
    let x = instance::build_ivar_load_raw(gen, sk_obj, struct_type, item_type, 0, "llvm_int");
    x.into_int_value()
}

pub fn box_bool<'run>(gen: &mut CodeGen<'run, '_>, b: bool) -> SkObj<'run> {
    let llvm_b = gen.context.bool_type().const_int(b as u64, false);
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_bool",
            &[llvm_b.as_basic_value_enum().into()],
            "sk_bool",
        )
        .unwrap()
        .into_pointer_value(),
    )
}

pub fn unbox_bool<'run>(
    gen: &mut CodeGen<'run, '_>,
    sk_obj: SkObj<'run>,
) -> inkwell::values::IntValue<'run> {
    let struct_type = llvm_struct::get(gen, &Erasure::nonmeta("Bool"));
    let item_type = gen.context.bool_type().into();
    let x = instance::build_ivar_load_raw(gen, sk_obj, struct_type, item_type, 0, "llvm_bool");
    x.into_int_value()
}
