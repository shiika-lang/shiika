use crate::codegen::{value::SkObj, CodeGen};
use shiika_core::names::{class_fullname, metaclass_fullname, TypeFullname};
use shiika_core::ty::{self, TermTy};
use skc_hir::MethodSignature;

pub fn create<'run>(
    gen: &mut CodeGen<'run, '_>,
    fullname: &TypeFullname,
    clsobj_ty: &TermTy,
    str_literal_idx: &usize,
    includes_modules: &bool,
    initializer: &Option<MethodSignature>,
) -> SkObj<'run> {
    debug_assert!(!fullname.is_meta());
    if fullname.0 == "Metaclass" {
        gen.gen_the_metaclass(str_literal_idx)
    } else {
        // Create metaclass object (eg. `#<metaclass Int>`) with `Metaclass.new`
        let the_metaclass = gen.gen_const_ref(&toplevel_const("Metaclass"), &ty::raw("Metaclass"));
        let receiver = gen.null_ptr();
        let vtable = gen
            .get_vtable_of_class(&class_fullname("Metaclass"))
            .as_object_ptr(gen);
        let wtable = SkObj::nullptr(gen);
        let metacls_obj = gen.call_method_func(
            &method_fullname_raw("Metaclass", "_new"),
            receiver,
            &vec![
                gen.gen_string_literal(str_literal_idx),
                vtable,
                wtable,
                gen.bitcast(the_metaclass, &ty::raw("Metaclass"), "as"),
                gen.null_ptr(),
            ],
            ty::raw("Metaclass"),
            "meta",
        );

        // Create the class object (eg. `#<class Int>`, which is the value of `::Int`)
        let receiver = gen.null_ptr();
        let vtable = gen
            .get_vtable_of_class(&fullname.meta_name())
            .as_object_ptr(gen);
        let wtable = SkObj::nullptr(gegen);
        let cls = gen.call_method_func(
            &method_fullname(metaclass_fullname("Class").into(), "_new"),
            receiver,
            &vec![
                gen.gen_string_literal(str_literal_idx),
                vtable,
                wtable,
                gen.bitcast(metacls_obj, &ty::raw("Metaclass"), "as"),
                gen.null_ptr(),
            ],
            ty::raw("Class"),
            "cls",
        );
        if *includes_modules {
            let fname = wtable::insert_wtable_func_name(&fullname.clone().to_class_fullname());
            gen.call_void_llvm_func(&llvm_func_name(fname), &[cls.0.into()], "_");
        }
        gen.call_class_level_initialize(cls.clone(), initializer);

        gen.bitcast(cls, clsobj_ty, "as")
    }
}

fn gen_the_metaclass<'run>(gen: &mut CodeGen<'run, '_>, str_literal_idx: &usize) -> SkObj<'run> {
    // We need a trick here to achieve `Metaclass.class == Metaclass`.
    let null = SkClassObj::nullptr(gen);
    let cls_obj = instance::allocate_sk_obj(gen, "Metaclass");
    instance::build_ivar_store_raw(
        gen,
        cls_obj.clone(),
        skc_corelib::class::IVAR_NAME_IDX,
        "@name",
        gen.gen_string_literal(str_literal_idx),
    );
    //todo gen.set_class_of_obj(&cls_obj, SkClassObj(cls_obj.0));
    cls_obj
}
