//! WTable (Witness Table) code generation for module method dispatch.
use crate::codegen::{self, constants, item, CodeGen};
use crate::names::FunctionName;
use anyhow::Result;
use inkwell::values::BasicValue;
use shiika_core::names::*;
use skc_hir::SkClass;
use skc_hir::SkTypes;

/// Declare llvm constants like `@shiika_wtable_Array_Enumerable`
pub fn declare_constants(gen: &mut CodeGen, sk_types: &SkTypes) {
    for sk_class in sk_types.sk_classes() {
        if !sk_class.wtable.is_empty() {
            declare_constant(gen, sk_class);
        }
    }
}

fn declare_constant(gen: &mut CodeGen, sk_class: &SkClass) {
    for (mod_name, method_names) in &sk_class.wtable.0 {
        let ary_type = gen.ptr_type().array_type(method_names.len() as u32);
        let cname = llvm_wtable_const_name(&sk_class.fullname(), mod_name);
        let global = gen.module.add_global(ary_type, None, &cname);
        global.set_constant(true);
    }
}

/// Set value of llvm constants like `@shiika_wtable_Array_Enumerable`
pub fn init_constants(gen: &mut CodeGen, sk_types: &SkTypes, _: codegen::item::MethodFuncs) {
    for sk_class in sk_types.sk_classes() {
        if !sk_class.wtable.is_empty() {
            init_constant(gen, sk_class);
        }
    }
}

pub fn init_constant(gen: &mut CodeGen, sk_class: &SkClass) {
    for (mod_name, method_names) in &sk_class.wtable.0 {
        let global = gen
            .module
            .get_global(&llvm_wtable_const_name(&sk_class.fullname(), mod_name))
            .unwrap();
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

//pub fn call_main_inserter(
//    gen: &mut CodeGen,
//) {
//    let fname = main_inserter_name();
//    let args = &[];
//    gen.call_llvm_func(&fname, args, "_");
//}

pub fn define_inserters(
    _const_global: item::ConstGlobal,
    gen: &mut CodeGen,
    sk_types: &SkTypes,
) -> Result<()> {
    define_class_inserters(_const_global, gen, sk_types)?;
    define_main_inserter(gen, sk_types)?;
    Ok(())
}

fn define_main_inserter(gen: &mut CodeGen, sk_types: &SkTypes) -> Result<()> {
    let fname = main_inserter_name();
    let function = {
        let ftype = gen.context.void_type().fn_type(&[], false);
        gen.module.add_function(&fname, ftype, None)
    };
    let basic_block = gen.context.append_basic_block(function, "");
    gen.builder.position_at_end(basic_block);

    call_all_inserters(gen, sk_types);

    gen.builder.build_return(None)?;
    Ok(())
}

fn call_all_inserters(gen: &mut CodeGen, sk_types: &SkTypes) {
    for sk_class in sk_types.sk_classes() {
        if !sk_class.wtable.is_empty() {
            call_inserter(gen, &sk_class.fullname());
        }
    }
}

/// Generate a call to inserter like `shiika_insert_Array_wtables(cls_obj)`
fn call_inserter(gen: &mut CodeGen, classname: &ClassFullname) {
    let cls_obj = constants::load(gen, &classname.to_const_fullname());
    let fname = insert_wtable_func_name(classname);
    let args = &[cls_obj.into()];
    gen.call_llvm_func(&fname, args, "_");
}

/// Insert wtable entries for all modules of the class
fn define_class_inserters(
    _const_global: item::ConstGlobal,
    gen: &mut CodeGen,
    sk_types: &SkTypes,
) -> Result<()> {
    for sk_class in sk_types.sk_classes() {
        if !sk_class.wtable.is_empty() {
            define_class_inserter(gen, sk_class)?;
        }
    }
    Ok(())
}

/// Define the inserter function like `shiika_insert_Array_wtables(cls_obj)`
fn define_class_inserter(gen: &mut CodeGen, sk_class: &SkClass) -> Result<()> {
    let function = {
        let fargs = &[gen.ptr_type().into()];
        let ftype = gen.context.void_type().fn_type(fargs, false);
        let fname = insert_wtable_func_name(&sk_class.fullname());
        gen.module.add_function(&fname, ftype, None)
    };
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
    gen.builder.build_return(None)?;
    Ok(())
}

/// Get the llvm constant like `@shiika_wtable_Array_Enumerable` as i8*
fn load_wtable_const<'a>(
    gen: &'a CodeGen,
    llvm_const_name: &str,
) -> inkwell::values::BasicValueEnum<'a> {
    let ptr = gen.module.get_global(llvm_const_name).unwrap_or_else(|| {
        panic!(
            "[BUG] WTable constant `{}` not declared. \
             Ensure declare_constants() was called before define_inserters(). \
             Check codegen.rs for correct phase ordering.",
            llvm_const_name
        )
    });
    gen.builder
        .build_bit_cast(ptr, gen.ptr_type(), "ary")
        .unwrap()
}

/// Name of llvm constant of a wtable
fn llvm_wtable_const_name(classname: &ClassFullname, modulename: &ModuleFullname) -> String {
    format!("shiika_wtable_{}_{}", classname.0, modulename.0)
}

pub fn main_inserter_name() -> String {
    "shiika_insert_all_wtables".to_string()
}

fn insert_wtable_func_name(cls: &ClassFullname) -> String {
    format!("shiika_insert_{}_wtables", cls)
}

/// Get the wtable key of a module
/// (Currently, the key is the address of a module object defined at runtime)
pub fn get_module_key<'run>(
    gen: &CodeGen<'run, '_>,
    fullname: &ModuleFullname,
) -> inkwell::values::IntValue<'run> {
    let type_obj = constants::load(gen, &fullname.clone().to_const_fullname());
    gen.builder
        .build_ptr_to_int(
            type_obj.into_pointer_value(),
            gen.context.i64_type(),
            "const_addr",
        )
        .unwrap()
}
