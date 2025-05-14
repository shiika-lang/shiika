use crate::codegen::CodeGen;
use shiika_core::names::ClassFullname;
use skc_mir;

/// Declare vtable constants
pub fn define(gen: &mut CodeGen, vtables: &skc_mir::VTables) {
    for (class_fullname, vtable) in vtables.iter() {
        let method_names = vtable.to_vec();
        let ary_type = gen.ptr_type().array_type(method_names.len() as u32);
        let tmp = llvm_vtable_const_name(class_fullname);
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

/// Name of llvm constant of a vtable
fn llvm_vtable_const_name(classname: &ClassFullname) -> String {
    format!("shiika_vtable_{}", classname.0)
}
