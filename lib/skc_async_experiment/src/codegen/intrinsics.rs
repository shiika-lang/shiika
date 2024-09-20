//! Intrinsics are functions defined directly by the compiler.
use crate::codegen::{instance, llvm_struct, value::SkObj, CodeGen};
use inkwell::values::BasicValue;

pub fn define(gen: &mut CodeGen) {
    define_box_int(gen);
    define_box_bool(gen);
}

fn define_box_int(gen: &mut CodeGen) {
    let fn_type = gen
        .ptr_type()
        .fn_type(&[gen.context.i64_type().into()], false);
    let function = gen
        .module
        .add_function("shiika_intrinsic_box_int", fn_type, None);
    let basic_block = gen.context.append_basic_block(function, "");
    gen.builder.position_at_end(basic_block);

    let i64_val = function.get_params()[0];
    let sk_int = instance::allocate_sk_obj(gen, "::Int");
    let struct_type = llvm_struct::get(gen, "::Int");
    instance::build_ivar_store_raw(gen, sk_int.clone(), &struct_type, 0, i64_val, "llvm_int");
    gen.builder.build_return(Some(&sk_int.0));
}

fn define_box_bool(gen: &mut CodeGen) {
    let fn_type = gen
        .ptr_type()
        .fn_type(&[gen.context.bool_type().into()], false);
    let function = gen
        .module
        .add_function("shiika_intrinsic_box_bool", fn_type, None);
    let basic_block = gen.context.append_basic_block(function, "");
    gen.builder.position_at_end(basic_block);

    let bool_val = function.get_params()[0];
    let sk_bool = instance::allocate_sk_obj(gen, "::Bool");
    let struct_type = llvm_struct::get(gen, "::Bool");
    instance::build_ivar_store_raw(gen, sk_bool.clone(), &struct_type, 0, bool_val, "llvm_bool");
    gen.builder.build_return(Some(&sk_bool.0));
}

pub fn box_int<'run>(gen: &mut CodeGen<'run, '_>, n: i64) -> SkObj<'run> {
    let llvm_n = gen.context.i64_type().const_int(n as u64, false);
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_int",
            &[llvm_n.as_basic_value_enum().into()],
            "sk_int",
        )
        .into_pointer_value(),
    )
}

pub fn unbox_int<'run>(
    gen: &mut CodeGen<'run, '_>,
    sk_obj: SkObj<'run>,
) -> inkwell::values::IntValue<'run> {
    let struct_type = llvm_struct::get(gen, "::Int");
    instance::build_ivar_load_raw(gen, sk_obj, struct_type, 0, "llvm_int").into_int_value()
}

pub fn box_bool<'run>(gen: &mut CodeGen<'run, '_>, b: bool) -> SkObj<'run> {
    let llvm_b = gen.context.bool_type().const_int(b as u64, false);
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_bool",
            &[llvm_b.as_basic_value_enum().into()],
            "sk_bool",
        )
        .into_pointer_value(),
    )
}
