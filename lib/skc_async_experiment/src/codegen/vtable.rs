use crate::codegen::{item, CodeGen};
use inkwell::values::BasicValue;
use shiika_core::names::ClassFullname;
use skc_hir::SkTypes;
use skc_mir;

/// Declare vtable constants
pub fn define(gen: &mut CodeGen, vtables: &skc_mir::VTables) {
    for (class_fullname, vtable) in vtables.iter() {
        let ary_type = gen.ptr_type().array_type(vtable.size() as u32);
        let const_name = llvm_vtable_const_name(class_fullname);
        let global = gen.module.add_global(ary_type, None, &const_name);
        global.set_constant(true);
    }
}

/// Insert method pointers into vtable constants
pub fn define_body(gen: &mut CodeGen, vtables: &skc_mir::VTables, _: item::MethodFuncs) {
    for (class_fullname, vtable) in vtables.iter() {
        let method_names = vtable.to_vec();
        let const_name = llvm_vtable_const_name(class_fullname);
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
                    .build_bitcast(func, gen.ptr_type(), "")
                    .into_pointer_value()
            })
            .collect::<Vec<_>>();
        global.set_initializer(&gen.ptr_type().const_array(&func_ptrs));
    }
}

/// Declare imported vtable constants
pub fn import(gen: &mut CodeGen, imported_types: &SkTypes) {
    imported_types.0.values().for_each(|sk_type| {
        let n_methods = sk_type.base().method_sigs.len();
        let ary_type = gen.ptr_type().array_type(n_methods as u32);
        let vtable_const_name = llvm_vtable_const_name(&sk_type.fullname().as_class_fullname());
        let global = gen.module.add_global(ary_type, None, &vtable_const_name);
        global.set_constant(true);
    });
}

/// Get vtable of the class of the given name
pub fn get<'run>(gen: &mut CodeGen<'run, '_>, classname: &ClassFullname) -> OpaqueVTableRef<'run> {
    let vtable_const_name = llvm_vtable_const_name(classname);
    let llvm_ary_ptr = gen
        .module
        .get_global(&vtable_const_name)
        .unwrap_or_else(|| panic!("global `{}' not found", &vtable_const_name))
        .as_pointer_value();
    OpaqueVTableRef::new(llvm_ary_ptr)
}

/// Get the function pointer at the given index in the vtable.
pub fn get_function<'run>(
    gen: &mut CodeGen<'run, '_>,
    vtable: OpaqueVTableRef<'run>,
    idx: usize,
) -> inkwell::values::PointerValue<'run> {
    gen.builder
        .build_extract_value(
            vtable.ptr.as_basic_value_enum().into_array_value(),
            idx as u32,
            "func_raw",
        )
        .unwrap()
        .into_pointer_value()
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
fn llvm_vtable_const_name(classname: &ClassFullname) -> String {
    format!("shiika_vtable_{}", classname.0)
}
