use crate::codegen::{instance, llvm_struct, value::SkObj, CodeGen};
use crate::mir;
use crate::prelude;
use shiika_core::ty::TermTy;

pub fn create<'run>(
    gen: &mut CodeGen<'run, '_>,
    the_ty: &TermTy,
    gen_name: inkwell::values::BasicValueEnum<'run>,
) -> SkObj<'run> {
    // REFACTOR: can we do this in MIR level?
    // this is implemented in codegen because HirClassLiteral appears on the rhs of const
    // assignment and mir::async_splitter cannot handle `Exprs` in inner position.

    debug_assert!(!the_ty.fullname.is_meta());
    if the_ty.fullname.0 == "Metaclass" {
        create_the_metaclass(gen, gen_name)
    } else {
        create_a_class(gen, the_ty, gen_name)
    }
}

fn create_the_metaclass<'run>(
    gen: &mut CodeGen<'run, '_>,
    gen_name: inkwell::values::BasicValueEnum<'run>,
) -> SkObj<'run> {
    // We need a trick here to achieve `Metaclass.class == Metaclass`.
    let cls_obj = instance::allocate_sk_obj(gen, "Metaclass");
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        &llvm_struct::of_ty(gen, &mir::Ty::raw("Metaclass")),
        prelude::IDX_CLASS_IVAR_NAME,
        gen_name,
        "@name",
    );
    //todo gen.set_class_of_obj(&cls_obj, SkClassObj(cls_obj.0));
    cls_obj
}

// Create a type object
fn create_a_class<'run>(
    gen: &mut CodeGen<'run, '_>,
    the_ty: &TermTy,
    gen_name: inkwell::values::BasicValueEnum<'run>,
) -> SkObj<'run> {
    let cls_obj = instance::allocate_sk_obj(gen, "Class");
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        &llvm_struct::of_ty(gen, &the_ty.meta_ty().into()),
        prelude::IDX_CLASS_IVAR_NAME,
        gen_name,
        "@name",
    );
    //todo gen.set_class_of_obj(&cls_obj, SkClassObj(cls_obj.0));
    cls_obj
}
