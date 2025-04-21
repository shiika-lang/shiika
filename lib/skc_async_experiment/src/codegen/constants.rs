use crate::codegen::CodeGen;
use crate::mir;
use shiika_core::names::ConstFullname;
use shiika_core::ty::TermTy;

pub fn declare_extern_consts(gen: &mut CodeGen, constants: Vec<(ConstFullname, TermTy)>) {
    for (fullname, _) in constants {
        let name = mir::mir_const_name(fullname);
        let global = gen.module.add_global(gen.ptr_type(), None, &name);
        global.set_linkage(inkwell::module::Linkage::External);
        // @init_::XX
        //let fn_type = gen.context.void_type().fn_type(&[], false);
        //gen.module
        //    .add_function(&const_initialize_func_name(fullname), fn_type, None);
    }
}

pub fn declare_const_globals(gen: &mut CodeGen, consts: &[(ConstFullname, TermTy)]) {
    for (fullname, _) in consts {
        let name = mir::mir_const_name(fullname.clone());
        let global = gen.module.add_global(gen.ptr_type(), None, &name);
        global.set_initializer(&gen.ptr_type().const_null());
    }
}
