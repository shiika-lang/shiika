use crate::codegen::CodeGen;
use skc_mir;

/// Declare vtable constants
pub fn define(gen: &mut CodeGen, vtables: &skc_mir::VTables) {
    for (class_fullname, vtable) in vtables.iter() {
        let method_names = vtable.to_vec();
        let ary_type = gen.ptr_type().array_type(method_names.len() as u32);
        let tmp = llvm_vtable_const_name(&class_fullname.0);
        let global = gen.module.add_global(ary_type, None, &tmp);
        global.set_constant(true);
        let func_ptrs = method_names
            .iter()
            .map(|name| {
                let func = gen
                    .get_llvm_func(&name.into())
                    .as_global_value()
                    .as_pointer_value();
                gen.builder
                    .build_bitcast(func, gen.ptr_type(), "")
                    .into_pointer_value()
            })
            .collect::<Vec<_>>();
        global.set_initializer(&gen.ptr_type().const_array(&func_ptrs));
    }
}

/// Get vtable of the class of the given name
pub fn get<'run>(gen: &mut CodeGen<'run, '_>, classname: &str) -> OpaqueVTableRef<'run> {
    let vtable_const_name = llvm_vtable_const_name(classname);
    let llvm_ary_ptr = gen
        .module
        .get_global(&vtable_const_name)
        .unwrap_or_else(|| panic!("[BUG] global `{}' not found", &vtable_const_name))
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
fn llvm_vtable_const_name(classname: &str) -> String {
    format!("shiika_vtable_{}", classname)
}
