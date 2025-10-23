use crate::codegen::{
    instance, llvm_struct, string_literal,
    value::{SkClassObj, SkObj},
    CodeGen,
};
use crate::prelude;
use shiika_core::ty::TermTy;

pub fn create<'run>(gen: &mut CodeGen<'run, '_>, the_ty: &TermTy) -> SkObj<'run> {
    // REFACTOR: can we do this in MIR level?
    // this is implemented in codegen because HirClassLiteral appears on the rhs of const
    // assignment and mir::async_splitter cannot handle `Exprs` in inner position.

    debug_assert!(!the_ty.fullname.is_meta());
    let type_obj = create_obj(gen, the_ty);

    if the_ty.fullname.0 == "Metaclass" {
        // Overwrite .class to achieve `Metaclass.class == Metaclass`.
        instance::set_class_obj(gen, &type_obj, SkClassObj(type_obj.0));
    }

    type_obj
}

/// Create a type object
fn create_obj<'run>(gen: &mut CodeGen<'run, '_>, the_ty: &TermTy) -> SkObj<'run> {
    let name_str = string_literal::generate(gen, &the_ty.fullname.0);
    let cls_obj = instance::allocate_sk_obj(gen, "Class");
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        &llvm_struct::of_ty(gen, &the_ty.meta_ty().into()),
        prelude::IDX_CLASS_IVAR_NAME,
        name_str,
        "@name",
    );
    cls_obj
}
