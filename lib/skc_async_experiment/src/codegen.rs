mod codegen_context;
mod compile;
mod type_object;
use crate::names::FunctionName;
mod constants;
mod instance;
mod intrinsics;
mod item;
mod llvm_struct;
mod string_literal;
mod value;
mod vtable;
use crate::mir;
use anyhow::{anyhow, Result};
use std::path::Path;

pub struct CodeGen<'run, 'ictx: 'run> {
    pub context: &'ictx inkwell::context::Context,
    pub module: &'run inkwell::module::Module<'ictx>,
    pub builder: &'run inkwell::builder::Builder<'ictx>,
    string_id: usize,
}

pub fn run<P: AsRef<Path>>(
    bc_path: P,
    opt_ll_path: Option<P>,
    mir: mir::CompilationUnit,
    is_bin: bool,
) -> Result<()> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let mut c = CodeGen {
        context: &context,
        module: &module,
        builder: &builder,
        string_id: 0,
    };
    c.compile_extern_funcs(mir.program.externs);
    constants::declare_extern_consts(&mut c, mir.imported_constants);
    constants::declare_const_globals(&mut c, &mir.program.constants);
    vtable::import(&mut c, &mir.imported_vtables);
    vtable::define(&mut c, &mir.vtables);
    llvm_struct::define(&mut c, &mir.program.classes);
    if is_bin {
        intrinsics::define(&mut c);
    }
    let method_funcs = c.compile_program(mir.program.funcs);
    vtable::define_body(&mut c, &mir.vtables, method_funcs);

    c.module.write_bitcode_to_path(bc_path.as_ref());
    if let Some(ll_path) = opt_ll_path {
        c.module
            .print_to_file(ll_path)
            .map_err(|llvm_str| anyhow!("{}", llvm_str.to_string()))?;
    }
    Ok(())
}

// Utilities used by codegen::*
impl<'run, 'ictx: 'run> CodeGen<'run, 'ictx> {
    fn llvm_type(&self, ty: &mir::Ty) -> inkwell::types::BasicTypeEnum<'ictx> {
        match ty {
            mir::Ty::Ptr => self.ptr_type().into(),
            mir::Ty::Any => self.context.i64_type().into(),
            mir::Ty::I1 => self.context.bool_type().into(),
            mir::Ty::Int64 => self.context.i64_type().into(),
            mir::Ty::ChiikaEnv | mir::Ty::RustFuture => self.ptr_type().into(),
            mir::Ty::Raw(s) => match s.as_str() {
                "Never" => panic!("Never is unexpected here"),
                _ => self.ptr_type().into(),
            },
            mir::Ty::Fun(_) => self.ptr_type().into(),
        }
    }

    fn ptr_type(&self) -> inkwell::types::PointerType<'ictx> {
        self.context.i8_type().ptr_type(Default::default())
    }

    /// Call llvm function (whose return type is not `void`)
    fn call_llvm_func(
        &self,
        func_name: &str,
        args: &[inkwell::values::BasicMetadataValueEnum<'run>],
        reg_name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let f = self
            .module
            .get_function(&func_name)
            .unwrap_or_else(|| panic!("llvm function {:?} not found", func_name));
        self.builder
            .build_direct_call(f, args, reg_name)
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    fn get_llvm_func(&self, name: &FunctionName) -> inkwell::values::FunctionValue<'run> {
        let mangled = name.mangle();
        self.module
            .get_function(&mangled)
            .unwrap_or_else(|| panic!("function `{:?}' not found", mangled))
    }
}
