mod codegen_context;
use crate::hir;
use anyhow::{anyhow, Result};
use codegen_context::CodeGenContext;
use inkwell::types::BasicType;
use std::path::Path;

pub struct SkValue<'run>(pub inkwell::values::BasicValueEnum<'run>);

pub struct CodeGen<'run, 'ictx: 'run> {
    pub context: &'ictx inkwell::context::Context,
    pub module: &'run inkwell::module::Module<'ictx>,
    pub builder: &'run inkwell::builder::Builder<'ictx>,
}

pub fn run<P: AsRef<Path>>(bc_path: P, opt_ll_path: Option<P>, prog: hir::Program) -> Result<()> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let c = CodeGen {
        context: &context,
        module: &module,
        builder: &builder,
    };
    c.compile_program(prog);

    c.module.write_bitcode_to_path(bc_path.as_ref());
    if let Some(ll_path) = opt_ll_path {
        c.module
            .print_to_file(ll_path)
            .map_err(|llvm_str| anyhow!("{}", llvm_str.to_string()))?;
    }
    Ok(())
}

impl<'run, 'ictx: 'run> CodeGen<'run, 'ictx> {
    fn compile_program(&self, prog: hir::Program) {
        for e in prog.externs {
            self.compile_extern(e);
        }
        for f in prog.funcs {
            self.compile_func(f);
        }
    }

    fn compile_extern(&self, ext: hir::Extern) {
        let func_type = self.llvm_function_type(&ext.fun_ty());
        self.module.add_function(&ext.name, func_type, None);
    }

    fn compile_func(&self, f: hir::Function) {
        let func_type = self.llvm_function_type(&f.fun_ty());
        let function = self.module.add_function(&f.name, func_type, None);
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let mut ctx = CodeGenContext {
            function: self.get_llvm_func(&f.name),
        };

        for stmt in &f.body_stmts {
            self.compile_expr(&mut ctx, stmt);
        }
    }

    fn compile_value_expr(
        &self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &hir::TypedExpr,
    ) -> SkValue<'run> {
        match self.compile_expr(ctx, texpr) {
            Some(v) => v,
            None => panic!("this expression does not have value"),
        }
    }

    fn compile_expr(
        &self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &hir::TypedExpr,
    ) -> Option<SkValue<'run>> {
        match &texpr.0 {
            hir::Expr::Number(n) => self.compile_number(*n),
            hir::Expr::PseudoVar(pvar) => self.compile_pseudo_var(pvar),
            //            hir::Expr::LVarRef(name) => self.compile_lvarref(block, lvars, name),
            hir::Expr::ArgRef(idx) => self.compile_argref(ctx, idx),
            hir::Expr::FuncRef(name) => self.compile_funcref(name),
            //            hir::Expr::OpCall(op, lhs, rhs) => {
            //                self.compile_op_call(blocks, block, lvars, op, lhs, rhs)
            //            }
            hir::Expr::FunCall(fexpr, arg_exprs) => self.compile_funcall(ctx, fexpr, arg_exprs),
            //            hir::Expr::If(cond, then, els) => {
            //                self.compile_if(blocks, block, lvars, cond, then, els, &texpr.1)
            //            }
            //            hir::Expr::Yield(expr) => self.compile_yield(blocks, block, lvars, expr),
            //            hir::Expr::While(cond, exprs) => self.compile_while(blocks, block, lvars, cond, exprs),
            //            hir::Expr::Alloc(name) => self.compile_alloc(block, lvars, name),
            //            hir::Expr::Assign(name, rhs) => self.compile_assign(blocks, block, lvars, name, rhs),
            hir::Expr::Return(val_expr) => self.compile_return(ctx, val_expr),
            hir::Expr::Cast(expr, cast_type) => self.compile_cast(ctx, expr, cast_type),
            //            hir::Expr::Br(expr, block_id) => self.compile_br(blocks, block, lvars, expr, block_id),
            //            hir::Expr::CondBr(cond, true_block_id, false_block_id) => {
            //                self.compile_cond_br(blocks, block, lvars, cond, true_block_id, false_block_id)
            //            }
            //            hir::Expr::BlockArgRef => self.compile_block_arg_ref(block),
            //            hir::Expr::Nop => Ok(None),
            _ => panic!("should be lowered before compiler.rs: {:?}", texpr.0),
        }
    }

    fn compile_number(&self, n: i64) -> Option<SkValue<'run>> {
        let i = self.context.i64_type().const_int(n as u64, false);
        Some(SkValue(i.into()))
    }

    fn compile_argref(&self, ctx: &mut CodeGenContext<'run>, idx: &usize) -> Option<SkValue<'run>> {
        let v = ctx.function.get_nth_param(*idx as u32).unwrap();
        Some(SkValue(v))
    }

    fn compile_funcref(&self, name: &str) -> Option<SkValue<'run>> {
        let f = self
            .get_llvm_func(name)
            .as_global_value()
            .as_pointer_value();
        Some(SkValue(f.into()))
    }

    fn compile_pseudo_var(&self, pseudo_var: &hir::PseudoVar) -> Option<SkValue<'run>> {
        let v = match pseudo_var {
            hir::PseudoVar::True => self.context.bool_type().const_int(1, false),
            hir::PseudoVar::False => self.context.bool_type().const_int(0, false),
            // Null is represented as `i64 0`
            hir::PseudoVar::Null => self.context.i64_type().const_int(0, false),
        };
        Some(SkValue(v.into()))
    }

    fn compile_funcall(
        &self,
        ctx: &mut CodeGenContext<'run>,
        fexpr: &hir::TypedExpr,
        arg_exprs: &[hir::TypedExpr],
    ) -> Option<SkValue<'run>> {
        let func = self.compile_value_expr(ctx, fexpr);
        let func_type = self.llvm_function_type(fexpr.1.as_fun_ty());
        let args = arg_exprs
            .iter()
            .map(|x| self.compile_value_expr(ctx, x).0.into())
            .collect::<Vec<_>>();
        let ret = self.builder.build_indirect_call(
            func_type,
            func.0.into_pointer_value(),
            &args,
            "calltmp",
        );
        Some(SkValue(ret.try_as_basic_value().left().unwrap()))
    }

    fn compile_return(
        &self,
        ctx: &mut CodeGenContext<'run>,
        val_expr: &hir::TypedExpr,
    ) -> Option<SkValue<'run>> {
        let val = self.compile_value_expr(ctx, val_expr);
        self.builder.build_return(Some(&val.0));
        None
    }

    fn compile_cast<'a>(
        &self,
        ctx: &mut CodeGenContext<'run>,
        cast_type: &hir::CastType,
        expr: &hir::TypedExpr,
    ) -> Option<SkValue<'run>> {
        let e = self.compile_value_expr(ctx, expr);
        let v = match cast_type {
            hir::CastType::AnyToFun(fun_ty) => self
                .builder
                .build_int_to_ptr(
                    e.0.into_int_value(),
                    self.llvm_function_type(fun_ty).ptr_type(Default::default()),
                    "inttoptr",
                )
                .into(),
            hir::CastType::AnyToInt | hir::CastType::IntToAny | hir::CastType::NullToAny => e.0,
            hir::CastType::FunToAny => self
                .builder
                .build_ptr_to_int(e.0.into_pointer_value(), self.int_type(), "ptrtoint")
                .into(),
        };
        Some(SkValue(v))
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

    fn get_llvm_func(&self, name: &str) -> inkwell::values::FunctionValue<'run> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("function `{:?}' not found", name))
    }
}
