use crate::codegen::{
    instance, llvm_struct, string_literal,
    value::{SkClassObj, SkObj},
    wtable, CodeGen,
};
use crate::prelude;
use shiika_core::ty::TermTy;

pub fn create<'run>(
    gen: &mut CodeGen<'run, '_>,
    the_ty: &TermTy,
    includes_modules: bool,
) -> SkObj<'run> {
    debug_assert!(!the_ty.fullname.is_meta());
    let type_obj = create_obj(gen, the_ty, includes_modules);

    if the_ty.fullname.0 == "Metaclass" {
        // Overwrite .class to achieve `Metaclass.class == Metaclass`.
        instance::set_class_obj(gen, &type_obj, SkClassObj(type_obj.0));
    } else {
        let meta_type_obj = {
            let o = create_obj(gen, &the_ty.meta_ty(), includes_modules);
            let the_metaclass = gen
                .compile_constref("::Metaclass")
                .expect("Metaclass class object not found")
                .into_pointer_value();
            instance::set_class_obj(gen, &o, SkClassObj(the_metaclass));
            o
        };
        instance::set_class_obj(gen, &type_obj, SkClassObj(meta_type_obj.0));
    }

    type_obj
}

/// Create a type object
fn create_obj<'run>(
    gen: &mut CodeGen<'run, '_>,
    the_ty: &TermTy,
    includes_modules: bool,
) -> SkObj<'run> {
    let name_str = string_literal::generate(gen, &the_ty.fullname.0);
    let cls_obj = instance::allocate_sk_obj(
        gen,
        if the_ty.is_metaclass() {
            "Metaclass"
        } else {
            "Class"
        },
    );
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        &llvm_struct::of_ty(gen, &the_ty.meta_ty().into()),
        prelude::IDX_CLASS_IVAR_NAME,
        name_str,
        "@name",
    );
    if includes_modules {
        wtable::call_inserter(gen, &the_ty.fullname.to_class_fullname(), cls_obj.0.clone());
    }
    cls_obj
}
