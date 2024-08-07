pub struct CodeGen<'hir: 'ictx, 'run, 'ictx: 'run> {
    pub generate_main: bool,
    pub context: &'ictx inkwell::context::Context,
    pub module: &'run inkwell::module::Module<'ictx>,
    pub builder: &'run inkwell::builder::Builder<'ictx>,
    pub i1_type: inkwell::types::IntType<'ictx>,
//    pub i8_type: inkwell::types::IntType<'ictx>,
//    pub ptr_type: inkwell::types::PointerType<'ictx>,
//    pub i32_type: inkwell::types::IntType<'ictx>,
//    pub i64_type: inkwell::types::IntType<'ictx>,
//    pub f64_type: inkwell::types::FloatType<'ictx>,
//    pub void_type: inkwell::types::VoidType<'ictx>,
}

pub fn run(_filename: &str, _src: &str, prog: hir::blocked::Program) -> Result<()> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let c = CodeGen {
        context: &context,
        module: &module,
        builder: &builder,
        i1_type: context.bool_type(),
    };
    c.compile_program(prog)?;
    Ok(())
}

impl<'hir: 'ictx, 'run, 'ictx: 'run> CodeGen<'hir, 'run, 'ictx> {
    fn compile_program(&self, prog: hir::blocked::Program) -> Result<()> {
        for e in prog.externs {
            self.compile_extern(e);
        }
        for f in prog.funcs {
            self.compile_func(f);
        }
    }

    fn compile_extern(&self, ext: hir::Extern) {
        let func_type = self.llvm_function_type(&ext.fun_ty());
        self.module
            .add_function(&ext.name, func_type, None);
    }

    fn compile_func(&self, f: hir::blocked::Function) {

        let mut blocks = vec![];
        for b in &f.body_blocks {
            let block_tys = b
                .param_tys
                .iter()
                .map(|x| self.mlir_type(x))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|x| (x, self.unknown_loc()))
                .collect::<Vec<_>>();
            blocks.push(ir::Block::new(&block_tys));
        }

        for (i, b) in f.body_blocks.iter().enumerate() {
            let current_block = &blocks[i];
            for stmt in &b.stmts {
                self.compile_expr(&blocks, current_block, &mut lvars, stmt)?;
            }
        }

        for block in blocks {
            region.append_block(block);
        }

        Ok(dialect::func::func(
            &self.context,
            self.str_attr(&f.name),
            TypeAttribute::new(self.function_type(&f.fun_ty())?.into()),
            region,
            &[],
            self.unknown_loc(),
        ))
    }

    fn llvm_function_type(&self, fun_ty: &hir::FunTy) -> inkwell::types::FunctionType<'ictx> {
        let param_tys = self.llvm_types(&fun_ty.param_tys)?;
        let ret_ty = self.llvm_type(&fun_ty.ret_ty)?;
        ret_ty.fn_type(&param_tys, false)
    }

    fn llvm_types(&self, tys: &[hir::Ty]) -> Result<Vec<inkwell::types::BasicTypeEnum<'ictx>>> {
        tys.iter().map(|x| self.llvm_type(x)).collect()
    }

    fn llvm_type(&self, ty: &hir::Ty) -> Result<inkwell::types::BasicTypeEnum<'ictx>> {
        let t = match ty {
            hir::Ty::Unknown => return Err(anyhow!("Unknown is unexpected here")),
            hir::Ty::Void => return Err(anyhow!("void is unexpected here")),
            hir::Ty::ChiikaEnv | hir::Ty::RustFuture => self.ptr_type().into(),
            hir::Ty::Any | hir::Ty::Int | hir::Ty::Null => self.int_type().into(),
            hir::Ty::Bool => self.bool_type().into(),
            hir::Ty::Fun(fun_ty) => self.llvm_function_type(fun_ty)?.into(),
        };
        Ok(t)
    }

    fn ptr_type(&self) -> inkwell::types::PointerType<'ictx> {
            self.context.i8_type().ptr_type(Default::default())
    }

    fn int_type(&self) -> inkwell::types::IntType<'ictx> {
        self.context.i64_type()
    }

    fn bool_type(&self) -> inkwell::types::IntType<'ictx> {
        self.context.bool_type()
    }
}
