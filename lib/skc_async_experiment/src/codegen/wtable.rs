use crate::codegen::{self, CodeGen};
use crate::mir;
use crate::names::FunctionName;
use inkwell::values::BasicValue;
use shiika_core::names::*;
use skc_hir::SkClass;
use skc_hir::SkTypes;

/// Define llvm constants like `@shiika_wtable_Array_Enumerable`
pub fn define_constants(gen: &mut CodeGen, sk_class: &SkClass, _: codegen::item::MethodFuncs) {
    for (mod_name, method_names) in &sk_class.wtable.0 {
        let ary_type = gen.ptr_type().array_type(method_names.len() as u32);
        let cname = llvm_wtable_const_name(&sk_class.fullname(), mod_name);
        let global = gen.module.add_global(ary_type, None, &cname);
        global.set_constant(true);
        let func_ptrs = method_names
            .iter()
            .map(|name| {
                let func_name: FunctionName = name.into();
                gen.get_llvm_func(&func_name)
                    .as_global_value()
                    .as_pointer_value()
            })
            .collect::<Vec<_>>();
        global.set_initializer(&gen.ptr_type().const_array(&func_ptrs));
    }
}

/// Insert wtable entries for all modules of the class
pub fn define_inserters(gen: &mut CodeGen, sk_types: &SkTypes) {
    for sk_class in sk_types.sk_classes() {
        if !sk_class.wtable.is_empty() {
            define_inserter(gen, sk_class);
        }
    }
}

fn define_inserter(gen: &mut CodeGen, sk_class: &SkClass) {
    let fargs = &[gen.ptr_type().into()];
    let ftype = gen.context.void_type().fn_type(fargs, false);
    let fname = insert_wtable_func_name(&sk_class.fullname());
    let function = gen.module.add_function(&fname, ftype, None);
    let basic_block = gen.context.append_basic_block(function, "");
    gen.builder.position_at_end(basic_block);

    for mod_name in sk_class.wtable.0.keys() {
        let key = get_module_key(gen, mod_name);
        let funcs = load_wtable_const(gen, &llvm_wtable_const_name(&sk_class.fullname(), mod_name));
        let cls = function.get_nth_param(0).unwrap();
        let len = sk_class.wtable.get_len(mod_name);
        let args = &[
            cls.into(),
            key.as_basic_value_enum().into(),
            funcs.into(),
            gen.context.i64_type().const_int(len as u64, false).into(),
        ];
        gen.call_llvm_func("shiika_insert_wtable", args, "_");
    }
    gen.builder.build_return(None);
}

/// Get the llvm constant like `@shiika_wtable_Array_Enumerable` as i8*
fn load_wtable_const<'a>(
    gen: &'a CodeGen,
    llvm_const_name: &str,
) -> inkwell::values::BasicValueEnum<'a> {
    let ptr = gen.module.get_global(llvm_const_name).unwrap_or_else(|| {
        panic!(
            "[BUG] global for constant `{}' not created",
            llvm_const_name
        )
    });
    gen.builder.build_bitcast(ptr, gen.ptr_type(), "ary")
}

/// Name of llvm constant of a wtable
fn llvm_wtable_const_name(classname: &ClassFullname, modulename: &ModuleFullname) -> String {
    format!("shiika_wtable_{}_{}", classname.0, modulename.0)
}

pub fn insert_wtable_func_name(cls: &ClassFullname) -> String {
    format!("insert_{}_wtables", cls)
}

/// Get the wtable key of a module
/// (Currently, the key is the address of a module object defined at runtime)
pub fn get_module_key<'run>(
    gen: &CodeGen<'run, '_>,
    fullname: &ModuleFullname,
) -> inkwell::values::IntValue<'run> {
    let const_name = mir::mir_const_name(fullname.clone().to_const_fullname());
    let global = gen
        .module
        .get_global(&const_name)
        .unwrap_or_else(|| panic!("global `{}' not found", const_name));

    gen.builder.build_ptr_to_int(
        global.as_pointer_value(),
        gen.context.i64_type(),
        "const_addr",
    )
}
