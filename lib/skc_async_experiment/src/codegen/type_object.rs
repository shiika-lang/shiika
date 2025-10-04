use crate::codegen::{instance, llvm_struct, string_literal, value::SkObj, CodeGen};
use crate::mir;
use crate::prelude;
use inkwell::values::BasicValue;
use shiika_core::names::TypeFullname;
use shiika_core::ty::{self, TermTy};
use skc_hir::MethodSignature;

pub fn create<'run>(
    gen: &mut CodeGen<'run, '_>,
    fullname: &TypeFullname,
    clsobj_ty: &TermTy,
    _str_literal_idx: &usize,
    _includes_modules: &bool,
    _initializer: &Option<MethodSignature>,
) -> SkObj<'run> {
    // REFACTOR: can we do this in MIR level?
    // this is implemented in codegen because HirClassLiteral appears on the rhs of const
    // assignment and mir::async_splitter cannot handle `Exprs` in inner position.

    debug_assert!(!fullname.is_meta());
    if fullname.0 == "Metaclass" {
        create_the_metaclass(gen)
    } else {
        create_a_class(gen, clsobj_ty)
    }
}

fn create_the_metaclass<'run>(gen: &mut CodeGen<'run, '_>) -> SkObj<'run> {
    // We need a trick here to achieve `Metaclass.class == Metaclass`.
    let s = string_literal::declare(gen, "Metaclass").as_basic_value_enum();
    let cls_obj = instance::allocate_sk_obj(gen, "Metaclass");
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        &llvm_struct::of_ty(gen, &mir::Ty::raw("Metaclass")),
        prelude::IDX_CLASS_IVAR_NAME,
        s,
        "@name",
    );
    //todo gen.set_class_of_obj(&cls_obj, SkClassObj(cls_obj.0));
    cls_obj
}

// Create a type object
fn create_a_class<'run>(gen: &mut CodeGen<'run, '_>, clsobj_ty: &TermTy) -> SkObj<'run> {
    let s = string_literal::declare(gen, &clsobj_ty.instance_ty().fullname.0).as_basic_value_enum();
    let cls_obj = instance::allocate_sk_obj(gen, "Class");
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        &llvm_struct::of_ty(gen, &clsobj_ty.clone().into()),
        prelude::IDX_CLASS_IVAR_NAME,
        s,
        "@name",
    );
    //todo gen.set_class_of_obj(&cls_obj, SkClassObj(cls_obj.0));
    cls_obj
}
