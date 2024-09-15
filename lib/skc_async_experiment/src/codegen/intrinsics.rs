//! Intrinsics are functions defined directly by the compiler.
use crate::codegen::{instance, llvm_struct, CodeGen, SkValue};
use inkwell::values::BasicValue;

pub fn define(gen: &mut CodeGen) {
    define_box_int(gen);
}

fn define_box_int(gen: &mut CodeGen) {
    let fn_type = gen.ptr_type().fn_type(&[gen.int_type().into()], false);
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

pub fn box_int<'run>(gen: &mut CodeGen<'run, '_>, n: i64) -> SkValue<'run> {
    let llvm_n = gen.context.i64_type().const_int(n as u64, false);
    SkValue(
        gen.call_llvm_func(
            "shiika_intrinsic_box_int",
            &[llvm_n.as_basic_value_enum().into()],
            "sk_int",
        )
        .into_pointer_value(),
    )
}
