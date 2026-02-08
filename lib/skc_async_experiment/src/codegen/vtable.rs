use crate::codegen::{item, CodeGen};
use shiika_core::ty::Erasure;
use skc_mir;

/// Declare vtable constants
pub fn define(gen: &mut CodeGen, vtables: &skc_mir::VTables) {
    for (class_fullname, vtable) in vtables.iter() {
        let ary_type = gen.ptr_type().array_type(vtable.size() as u32);
        let const_name = llvm_vtable_const_name(&class_fullname.erasure());
        let global = gen.module.add_global(ary_type, None, &const_name);
        global.set_constant(true);
    }
}

/// Insert method pointers into vtable constants
pub fn define_body(gen: &mut CodeGen, vtables: &skc_mir::VTables, _: item::MethodFuncs) {
    for (class_fullname, vtable) in vtables.iter() {
        let method_names = vtable.to_vec();
        let const_name = llvm_vtable_const_name(&class_fullname.erasure());
        let global = gen
            .module
            .get_global(&const_name)
            .unwrap_or_else(|| panic!("global `{}' not found", &const_name));
        global.set_constant(true);
        let func_ptrs = method_names
            .iter()
            .map(|name| {
                let func = gen
                    .get_llvm_func(&name.into())
                    .as_global_value()
                    .as_pointer_value();
                gen.builder
                    .build_bit_cast(func, gen.ptr_type(), "")
                    .unwrap()
                    .into_pointer_value()
            })
            .collect::<Vec<_>>();
        global.set_initializer(&gen.ptr_type().const_array(&func_ptrs));
    }
}

/// Declare imported vtable constants
pub fn import(gen: &mut CodeGen, imported_vtables: &skc_mir::VTables) {
    imported_vtables
        .iter()
        .for_each(|(class_fullname, vtable)| {
            let n_methods = vtable.size();
            let ary_type = gen.ptr_type().array_type(n_methods as u32);
            let vtable_const_name = llvm_vtable_const_name(&class_fullname.erasure());
            let global = gen.module.add_global(ary_type, None, &vtable_const_name);
            global.set_constant(true);
        });
}

/// Get vtable of the class of the given name
pub fn get<'run>(gen: &mut CodeGen<'run, '_>, classname: &Erasure) -> OpaqueVTableRef<'run> {
    let vtable_const_name = llvm_vtable_const_name(classname);
    let llvm_ary_ptr = gen
        .module
        .get_global(&vtable_const_name)
        .unwrap_or_else(|| panic!("global `{}' not found", &vtable_const_name))
        .as_pointer_value();
    OpaqueVTableRef::new(llvm_ary_ptr)
}

/// Reference to vtable where its length is unknown.
#[derive(Debug)]
pub struct OpaqueVTableRef<'run> {
    pub ptr: inkwell::values::PointerValue<'run>,
}

impl<'run> OpaqueVTableRef<'run> {
    pub fn new(ptr: inkwell::values::PointerValue<'run>) -> OpaqueVTableRef<'run> {
        OpaqueVTableRef { ptr }
    }
}

/// Name of llvm constant of a vtable
fn llvm_vtable_const_name(classname: &Erasure) -> String {
    let meta = if classname.is_meta { "Meta:" } else { "" };
    format!("shiika_vtable_{}{}", meta, &classname.base_name)
}
