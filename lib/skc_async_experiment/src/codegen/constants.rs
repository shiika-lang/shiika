use crate::codegen::value::SkObj;
use crate::codegen::CodeGen;
use crate::mir;
use shiika_core::names::ConstFullname;
use shiika_core::ty::TermTy;

pub fn declare_extern_consts(gen: &mut CodeGen, constants: Vec<(ConstFullname, TermTy)>) {
    for (fullname, _) in constants {
        let name = mir::mir_const_name(fullname);
        let global = gen.module.add_global(gen.ptr_type(), None, &name);
        global.set_linkage(inkwell::module::Linkage::External);
    }
}

pub fn declare_const_globals(gen: &mut CodeGen, consts: &[(ConstFullname, TermTy)]) {
    for (fullname, _) in consts {
        let name = mir::mir_const_name(fullname.clone());
        let global = gen.module.add_global(gen.ptr_type(), None, &name);
        global.set_initializer(&gen.ptr_type().const_null());
    }
}

pub fn load<'run>(gen: &mut CodeGen<'run, '_>, name: &ConstFullname) -> SkObj<'run> {
    let class_const_name = mir::mir_const_name(name.clone());
    let class_obj_addr = gen
        .module
        .get_global(&class_const_name)
        .unwrap_or_else(|| panic!("global `{}' not found", class_const_name))
        .as_pointer_value();
    let t = gen.ptr_type();
    SkObj(
        gen.builder
            .build_load(t, class_obj_addr, "class_obj")
            .into_pointer_value(),
    )
}
