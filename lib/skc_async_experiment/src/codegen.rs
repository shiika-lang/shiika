use crate::hir;
use anyhow::Result;
use inkwell::types::BasicType;

pub struct SkValue<'run>(pub inkwell::values::BasicValueEnum<'run>);

pub struct CodeGen<'run, 'ictx: 'run> {
    pub context: &'ictx inkwell::context::Context,
    pub module: &'run inkwell::module::Module<'ictx>,
    pub builder: &'run inkwell::builder::Builder<'ictx>,
}

pub fn run(_filename: &str, _src: &str, prog: hir::Program) -> Result<()> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let c = CodeGen {
        context: &context,
        module: &module,
        builder: &builder,
    };
    c.compile_program(prog)?;
    Ok(())
}

impl<'run, 'ictx: 'run> CodeGen<'run, 'ictx> {
    fn compile_program(&self, prog: hir::Program) -> Result<()> {
        for e in prog.externs {
            self.compile_extern(e);
        }
        for f in prog.funcs {
            self.compile_func(f)?;
        }
        Ok(())
    }

    fn compile_extern(&self, ext: hir::Extern) {
        let func_type = self.llvm_function_type(&ext.fun_ty());
        self.module.add_function(&ext.name, func_type, None);
    }

    fn compile_func(&self, f: hir::Function) -> Result<()> {
        for stmt in &f.body_stmts {
            self.compile_expr(stmt)?;
        }
        Ok(())
    }

    fn compile_value_expr(&self, texpr: &hir::TypedExpr) -> Result<SkValue<'run>> {
        match self.compile_expr(texpr)? {
            Some(v) => Ok(v),
            None => panic!("this expression does not have value"),
        }
    }

    fn compile_expr(&self, texpr: &hir::TypedExpr) -> Result<Option<SkValue<'run>>> {
        match &texpr.0 {
            hir::Expr::Number(n) => self.compile_number(*n),
            hir::Expr::PseudoVar(pvar) => self.compile_pseudo_var(pvar),
            //            hir::Expr::LVarRef(name) => self.compile_lvarref(block, lvars, name),
            //            hir::Expr::ArgRef(idx) => self.compile_argref(blocks, idx),
            //            hir::Expr::FuncRef(name) => {
            //                let hir::Ty::Fun(fun_ty) = &texpr.1 else {
            //                    return Err(anyhow!("[BUG] not a function: {:?}", texpr.1));
            //                };
            //                self.compile_funcref(block, name, &fun_ty)
            //            }
            //            hir::Expr::OpCall(op, lhs, rhs) => {
            //                self.compile_op_call(blocks, block, lvars, op, lhs, rhs)
            //            }
            //            hir::Expr::FunCall(fexpr, arg_exprs) => {
            //                self.compile_funcall(blocks, block, lvars, fexpr, arg_exprs)
            //            }
            //            hir::Expr::If(cond, then, els) => {
            //                self.compile_if(blocks, block, lvars, cond, then, els, &texpr.1)
            //            }
            //            hir::Expr::Yield(expr) => self.compile_yield(blocks, block, lvars, expr),
            //            hir::Expr::While(cond, exprs) => self.compile_while(blocks, block, lvars, cond, exprs),
            //            hir::Expr::Alloc(name) => self.compile_alloc(block, lvars, name),
            //            hir::Expr::Assign(name, rhs) => self.compile_assign(blocks, block, lvars, name, rhs),
            hir::Expr::Return(val_expr) => self.compile_return(val_expr),
            //            hir::Expr::Cast(expr, cast_type) => {
            //                self.compile_cast(blocks, block, lvars, expr, cast_type)
            //            }
            //            hir::Expr::Br(expr, block_id) => self.compile_br(blocks, block, lvars, expr, block_id),
            //            hir::Expr::CondBr(cond, true_block_id, false_block_id) => {
            //                self.compile_cond_br(blocks, block, lvars, cond, true_block_id, false_block_id)
            //            }
            //            hir::Expr::BlockArgRef => self.compile_block_arg_ref(block),
            //            hir::Expr::Nop => Ok(None),
            _ => panic!("should be lowered before compiler.rs: {:?}", texpr.0),
        }
    }

    fn compile_number(&self, n: i64) -> Result<Option<SkValue<'run>>> {
        Ok(Some(SkValue(
            self.context.i64_type().const_int(n as u64, false).into(),
        )))
    }

    fn compile_pseudo_var(&self, pseudo_var: &hir::PseudoVar) -> Result<Option<SkValue<'run>>> {
        let v = match pseudo_var {
            hir::PseudoVar::True => self.context.bool_type().const_int(1, false),
            hir::PseudoVar::False => self.context.bool_type().const_int(0, false),
            // Null is represented as `i64 0`
            hir::PseudoVar::Null => self.context.i64_type().const_int(0, false),
        };
        Ok(Some(SkValue(v.into())))
    }

    fn compile_return(&self, val_expr: &hir::TypedExpr) -> Result<Option<SkValue<'run>>> {
        let val = self.compile_value_expr(val_expr)?;
        self.builder.build_return(Some(&val.0));
        Ok(None)
    }

    fn llvm_function_type(&self, fun_ty: &hir::FunTy) -> inkwell::types::FunctionType<'ictx> {
        let param_tys = self.llvm_types(&fun_ty.param_tys);
        let ret_ty = self.llvm_type(&fun_ty.ret_ty);
        ret_ty.fn_type(&param_tys, false)
    }

    fn llvm_types(&self, tys: &[hir::Ty]) -> Vec<inkwell::types::BasicMetadataTypeEnum<'ictx>> {
        tys.iter().map(|x| self.llvm_type(x).into()).collect()
    }

    fn llvm_type(&self, ty: &hir::Ty) -> inkwell::types::BasicTypeEnum<'ictx> {
        match ty {
            hir::Ty::Unknown => panic!("Unknown is unexpected here"),
            hir::Ty::Void => panic!("void is unexpected here"),
            hir::Ty::ChiikaEnv | hir::Ty::RustFuture => self.ptr_type().into(),
            hir::Ty::Any | hir::Ty::Int | hir::Ty::Null => self.int_type().into(),
            hir::Ty::Bool => self.bool_type().into(),
            hir::Ty::Fun(_) => self.ptr_type().into(),
        }
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
