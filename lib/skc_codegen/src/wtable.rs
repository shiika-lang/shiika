use crate::utils::{llvm_func_name, method_func_name};
use crate::CodeGen;
use inkwell::values::*;
use shiika_core::{names::*, ty};
use skc_hir::SkClass;

/// Define llvm constants like `@shiika_wtable_Array_Enumerable`
pub fn gen_wtable_constants(code_gen: &CodeGen, sk_class: &SkClass) {
    for (mod_name, method_names) in &sk_class.wtable.0 {
        let ary_type = code_gen.ptr_type.array_type(method_names.len() as u32);
        let cname = llvm_wtable_const_name(&sk_class.fullname(), mod_name);
        let global = code_gen.module.add_global(ary_type, None, &cname);
        global.set_constant(true);
        let func_ptrs = method_names
            .iter()
            .map(|name| {
                let func = code_gen
                    .get_llvm_func(&method_func_name(name))
                    .as_global_value()
                    .as_pointer_value();
                code_gen
                    .builder
                    .build_bitcast(func, code_gen.ptr_type, "")
                    .into_pointer_value()
            })
            .collect::<Vec<_>>();
        global.set_initializer(&code_gen.ptr_type.const_array(&func_ptrs));
    }
}

/// Define `@insert_XX_wtables()` for the class
pub fn gen_insert_wtable(code_gen: &CodeGen, sk_class: &SkClass) {
    let fargs = &[code_gen.llvm_type(&ty::raw("Class")).into()];
    let ftype = code_gen.void_type.fn_type(fargs, false);
    let fname = insert_wtable_func_name(&sk_class.fullname());
    let function = code_gen.module.add_function(&fname, ftype, None);
    let basic_block = code_gen.context.append_basic_block(function, "");
    code_gen.builder.position_at_end(basic_block);

    for mod_name in sk_class.wtable.0.keys() {
        let key = code_gen.get_const_addr_int(&mod_name.to_const_fullname());
        let funcs = load_wtable_const(
            code_gen,
            &llvm_wtable_const_name(&sk_class.fullname(), mod_name),
        );
        let cls = code_gen.get_nth_param(ty::raw("Class"), &function, 0);
        let len = sk_class.wtable.get_len(mod_name);
        let args = &[
            cls.into_i8ptr(code_gen).into(),
            key.as_basic_value_enum().into(),
            funcs.into(),
            code_gen.i64_type.const_int(len as u64, false).into(),
        ];
        code_gen.call_llvm_func(&llvm_func_name("shiika_insert_wtable"), args, "_");
    }
    code_gen.builder.build_return(None);
}

/// Get the llvm constant like `@shiika_wtable_Array_Enumerable` as i8*
fn load_wtable_const<'a>(
    code_gen: &'a CodeGen,
    llvm_const_name: &str,
) -> inkwell::values::BasicValueEnum<'a> {
    let ptr = code_gen
        .module
        .get_global(llvm_const_name)
        .unwrap_or_else(|| {
            panic!(
                "[BUG] global for constant `{}' not created",
                llvm_const_name
            )
        });
    code_gen
        .builder
        .build_bitcast(ptr, code_gen.ptr_type, "ary")
}

/// Name of llvm constant of a wtable
fn llvm_wtable_const_name(classname: &ClassFullname, modulename: &ModuleFullname) -> String {
    format!("shiika_wtable_{}_{}", classname.0, modulename.0)
}

pub fn insert_wtable_func_name(cls: &ClassFullname) -> String {
    format!("insert_{}_wtables", cls)
}
