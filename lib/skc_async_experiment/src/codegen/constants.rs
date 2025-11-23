use crate::codegen::item;
use crate::codegen::CodeGen;
use shiika_core::names::ConstFullname;
use shiika_core::ty::TermTy;
use shiika_ffi_mangle::mangle_const;

pub fn declare_extern_consts(gen: &mut CodeGen, constants: Vec<(ConstFullname, TermTy)>) {
    for (fullname, _) in constants {
        let name = llvm_const_name(&fullname);
        let global = gen.module.add_global(gen.ptr_type(), None, &name);
        global.set_linkage(inkwell::module::Linkage::External);
    }
}

pub fn declare_const_globals(
    gen: &mut CodeGen,
    consts: &[(ConstFullname, TermTy)],
) -> item::ConstGlobal {
    for (fullname, _) in consts {
        let name = llvm_const_name(fullname);
        let global = gen.module.add_global(gen.ptr_type(), None, &name);
        global.set_initializer(&gen.ptr_type().const_null());
    }
    item::ConstGlobal()
}

pub fn load<'run>(
    gen: &CodeGen<'run, '_>,
    name: &ConstFullname,
) -> inkwell::values::BasicValueEnum<'run> {
    let obj_addr = get_global(gen, name).as_pointer_value();
    let t = gen.ptr_type();
    gen.builder.build_load(t, obj_addr, "const_value").unwrap()
}

pub fn get_global<'run>(
    gen: &CodeGen<'run, '_>,
    name: &ConstFullname,
) -> inkwell::values::GlobalValue<'run> {
    let const_name = llvm_const_name(name);
    gen.module
        .get_global(&const_name)
        .unwrap_or_else(|| panic!("global `{}' not found", const_name))
}

fn llvm_const_name(fullname: &ConstFullname) -> String {
    mangle_const(&fullname.0)
}
