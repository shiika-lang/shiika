//! Intrinsics are functions defined directly by the compiler.
use crate::codegen::{instance, llvm_struct, value::SkObj, CodeGen};
use anyhow::Result;
use inkwell::values::BasicValue;
use shiika_core::ty::Erasure;

pub fn define(gen: &mut CodeGen) -> Result<()> {
    define_box_int(gen)?;
    define_box_bool(gen)?;
    define_box_float(gen)?;
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
    // Boxed `true`/`false` are two global singletons (zero-alloc boxing, and
    // `equal?` identity). They are lazily initialized on the first call, which
    // avoids depending on constant-init ordering (allocation needs `::Bool`).
    let ptr_ty = gen.ptr_type();
    let g_true = gen.module.add_global(ptr_ty, None, "shiika_bool_true");
    g_true.set_initializer(&ptr_ty.const_null());
    g_true.set_linkage(inkwell::module::Linkage::Internal);
    let g_false = gen.module.add_global(ptr_ty, None, "shiika_bool_false");
    g_false.set_initializer(&ptr_ty.const_null());
    g_false.set_linkage(inkwell::module::Linkage::Internal);

    let fn_type = ptr_ty.fn_type(&[gen.context.bool_type().into()], false);
    let function = gen
        .module
        .add_function("shiika_intrinsic_box_bool", fn_type, None);
    let entry = gen.context.append_basic_block(function, "entry");
    let init_bb = gen.context.append_basic_block(function, "init");
    let done_bb = gen.context.append_basic_block(function, "done");

    // entry: initialize the singletons if not done yet.
    gen.builder.position_at_end(entry);
    let cur = gen
        .builder
        .build_load(ptr_ty, g_true.as_pointer_value(), "cur")
        .unwrap();
    let uninit = gen
        .builder
        .build_is_null(cur.into_pointer_value(), "uninit")
        .unwrap();
    gen.builder
        .build_conditional_branch(uninit, init_bb, done_bb)?;

    // init: allocate one boxed Bool for each value and store into the globals.
    gen.builder.position_at_end(init_bb);
    let struct_type = llvm_struct::get(gen, &Erasure::nonmeta("Bool"));
    let t = instance::allocate_sk_obj(gen, &Erasure::nonmeta("Bool"))?;
    let true_val = gen.context.bool_type().const_int(1, false);
    instance::build_ivar_store_raw(
        gen,
        t.clone(),
        &struct_type,
        0,
        true_val.into(),
        "llvm_bool",
    )?;
    gen.builder.build_store(g_true.as_pointer_value(), t.0)?;
    let f = instance::allocate_sk_obj(gen, &Erasure::nonmeta("Bool"))?;
    let false_val = gen.context.bool_type().const_int(0, false);
    instance::build_ivar_store_raw(
        gen,
        f.clone(),
        &struct_type,
        0,
        false_val.into(),
        "llvm_bool",
    )?;
    gen.builder.build_store(g_false.as_pointer_value(), f.0)?;
    gen.builder.build_unconditional_branch(done_bb)?;

    // done: return the singleton selected by the argument.
    gen.builder.position_at_end(done_bb);
    let bool_val = function.get_params()[0].into_int_value();
    let tp = gen
        .builder
        .build_load(ptr_ty, g_true.as_pointer_value(), "t")
        .unwrap();
    let fp = gen
        .builder
        .build_load(ptr_ty, g_false.as_pointer_value(), "f")
        .unwrap();
    let r = gen
        .builder
        .build_select(bool_val, tp, fp, "sk_bool")
        .unwrap();
    gen.builder.build_return(Some(&r))?;
    Ok(())
}

fn define_box_float(gen: &mut CodeGen) -> Result<()> {
    let fn_type = gen
        .ptr_type()
        .fn_type(&[gen.context.f64_type().into()], false);
    let function = gen
        .module
        .add_function("shiika_intrinsic_box_float", fn_type, None);
    let basic_block = gen.context.append_basic_block(function, "");
    gen.builder.position_at_end(basic_block);

    let f64_val = function.get_params()[0];
    let sk_float = instance::allocate_sk_obj(gen, &Erasure::nonmeta("Float"))?;
    let struct_type = llvm_struct::get(gen, &Erasure::nonmeta("Float"));
    instance::build_ivar_store_raw(
        gen,
        sk_float.clone(),
        &struct_type,
        0,
        f64_val,
        "llvm_float",
    )?;
    gen.builder.build_return(Some(&sk_float.0))?;
    Ok(())
}

pub fn box_int_value<'run>(
    gen: &mut CodeGen<'run, '_>,
    n: inkwell::values::IntValue<'run>,
) -> SkObj<'run> {
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_int",
            &[n.as_basic_value_enum().into()],
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

pub fn box_float<'run>(gen: &mut CodeGen<'run, '_>, n: f64) -> SkObj<'run> {
    let llvm_n = gen.context.f64_type().const_float(n);
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_float",
            &[llvm_n.as_basic_value_enum().into()],
            "sk_float",
        )
        .unwrap()
        .into_pointer_value(),
    )
}

pub fn box_float_value<'run>(
    gen: &mut CodeGen<'run, '_>,
    n: inkwell::values::FloatValue<'run>,
) -> SkObj<'run> {
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_float",
            &[n.as_basic_value_enum().into()],
            "sk_float",
        )
        .unwrap()
        .into_pointer_value(),
    )
}

pub fn unbox_float<'run>(
    gen: &mut CodeGen<'run, '_>,
    sk_obj: SkObj<'run>,
) -> inkwell::values::FloatValue<'run> {
    let struct_type = llvm_struct::get(gen, &Erasure::nonmeta("Float"));
    let item_type = gen.context.f64_type().into();
    let x = instance::build_ivar_load_raw(gen, sk_obj, struct_type, item_type, 0, "llvm_float");
    x.into_float_value()
}

pub fn box_bool<'run>(gen: &mut CodeGen<'run, '_>, b: bool) -> SkObj<'run> {
    let llvm_b = gen.context.bool_type().const_int(b as u64, false);
    box_bool_value(gen, llvm_b)
}

pub fn box_bool_value<'run>(
    gen: &mut CodeGen<'run, '_>,
    b: inkwell::values::IntValue<'run>,
) -> SkObj<'run> {
    SkObj(
        gen.call_llvm_func(
            "shiika_intrinsic_box_bool",
            &[b.as_basic_value_enum().into()],
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
