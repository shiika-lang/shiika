mod codegen_context;
mod compile;
pub mod prelude;
mod type_object;
use crate::names::FunctionName;
mod constants;
mod instance;
mod intrinsics;
mod item;
mod llvm_struct;
mod sanity_check;
mod string_literal;
mod value;
mod vtable;
pub mod wtable;
use crate::mir;
use anyhow::{anyhow, Result};
use inkwell::values::AnyValue;
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

    let mut gen = CodeGen {
        context: &context,
        module: &module,
        builder: &builder,
        string_id: 0,
    };
    gen.compile_extern_funcs(mir.program.externs);
    constants::declare_extern_consts(&mut gen, mir.imported_constants);
    let _const_global_ = constants::declare_const_globals(&mut gen, &mir.program.constants);
    wtable::declare_constants(&mut gen, &mir.sk_types);
    vtable::import(&mut gen, &mir.imported_vtables);
    vtable::define(&mut gen, &mir.vtables);

    llvm_struct::define(&mut gen, &mir.program.classes);
    if is_bin {
        intrinsics::define(&mut gen)?;
    }

    let _method_funcs_ = gen.compile_program(mir.program.funcs);
    wtable::init_constants(&mut gen, &mir.sk_types, _method_funcs_);
    vtable::define_body(&mut gen, &mir.vtables, _method_funcs_);

    sanity_check::run(&gen.module)?;

    gen.module.write_bitcode_to_path(bc_path.as_ref());
    if let Some(ll_path) = opt_ll_path {
        gen.module
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
            mir::Ty::CVoid => panic!("CVoid is not a BasicTypeEnum"),
            mir::Ty::ChiikaEnv | mir::Ty::RustFuture => self.ptr_type().into(),
            mir::Ty::Raw(s) => match s.as_str() {
                "Never" => panic!("Never is unexpected here"),
                _ => self.ptr_type().into(),
            },
            mir::Ty::Fun(_) => self.ptr_type().into(),
        }
    }

    fn ptr_type(&self) -> inkwell::types::PointerType<'ictx> {
        self.context.ptr_type(Default::default())
    }

    /// Call llvm function. Returns `None` for void functions.
    fn call_llvm_func(
        &self,
        func_name: &str,
        args: &[inkwell::values::BasicMetadataValueEnum<'run>],
        reg_name: &str,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let f = self
            .module
            .get_function(&func_name)
            .unwrap_or_else(|| panic!("llvm function {:?} not found", func_name));
        let call_result = self.builder.build_direct_call(f, args, reg_name).unwrap();
        call_result.set_tail_call(true);
        if call_result.try_as_basic_value().is_basic() {
            Some(call_result.as_any_value_enum().try_into().unwrap())
        } else {
            None
        }
    }

    fn get_llvm_func(&self, name: &FunctionName) -> inkwell::values::FunctionValue<'run> {
        let mangled = name.mangle();
        self.module
            .get_function(&mangled)
            .unwrap_or_else(|| panic!("function `{:?}' not found", mangled))
    }
}
