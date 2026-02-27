use crate::codegen::prelude;
use crate::codegen::{
    instance, llvm_struct, string_literal,
    value::{SkClassObj, SkObj},
    CodeGen,
};
use anyhow::Result;
use shiika_core::names::ConstFullname;
use shiika_core::ty::{Erasure, TermTy};

pub fn create<'run>(gen: &mut CodeGen<'run, '_>, the_ty: &TermTy) -> Result<SkObj<'run>> {
    debug_assert!(!the_ty.fullname.is_meta());
    let type_obj = create_type_obj(gen, the_ty)?;

    if the_ty.fullname.0 == "Metaclass" {
        // Overwrite .class to achieve `Metaclass.class == Metaclass`.
        instance::set_class_obj(gen, &type_obj, SkClassObj(type_obj.0))?;
    } else {
        let meta_type_obj = {
            let o = create_type_obj(gen, &the_ty.meta_ty())?;
            let the_metaclass = gen
                .compile_constref(&ConstFullname::toplevel("Metaclass"))
                .unwrap()
                .into_pointer_value();
            instance::set_class_obj(gen, &o, SkClassObj(the_metaclass))?;
            o
        };
        instance::set_class_obj(gen, &type_obj, SkClassObj(meta_type_obj.0))?;
    }

    Ok(type_obj)
}

/// Create a type object
/// When `Meta:Foo` is given, create a metaclass object.
fn create_type_obj<'run>(gen: &mut CodeGen<'run, '_>, the_ty: &TermTy) -> Result<SkObj<'run>> {
    let name_str = string_literal::generate(gen, &the_ty.fullname.0);
    let clsname = if the_ty.is_metaclass() {
        Erasure::the_metaclass()
    } else {
        Erasure::nonmeta("Class")
    };
    let cls_obj = instance::allocate_sk_obj(gen, &clsname)?;
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        &llvm_struct::of_ty(gen, &the_ty.meta_ty().into()),
        prelude::IDX_CLASS_IVAR_NAME,
        name_str,
        "@name",
    )?;
    Ok(cls_obj)
}
