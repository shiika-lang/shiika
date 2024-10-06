mod codegen_context;
mod instance;
mod intrinsics;
mod llvm_struct;
mod value;
use crate::hir;
use anyhow::{anyhow, Result};
use codegen_context::CodeGenContext;
use inkwell::types::BasicType;
use std::path::Path;
use value::SkObj;

pub struct CodeGen<'run, 'ictx: 'run> {
    pub context: &'ictx inkwell::context::Context,
    pub module: &'run inkwell::module::Module<'ictx>,
    pub builder: &'run inkwell::builder::Builder<'ictx>,
}

pub fn run<P: AsRef<Path>>(bc_path: P, opt_ll_path: Option<P>, prog: hir::Program) -> Result<()> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let mut c = CodeGen {
        context: &context,
        module: &module,
        builder: &builder,
    };
    c.compile_externs(prog.externs);
    llvm_struct::define(&mut c);
    intrinsics::define(&mut c);
    c.compile_program(prog.funcs);

    c.module.write_bitcode_to_path(bc_path.as_ref());
    if let Some(ll_path) = opt_ll_path {
        c.module
            .print_to_file(ll_path)
            .map_err(|llvm_str| anyhow!("{}", llvm_str.to_string()))?;
    }
    Ok(())
}

impl<'run, 'ictx: 'run> CodeGen<'run, 'ictx> {
    fn compile_externs(&mut self, externs: Vec<hir::Extern>) {
        for e in externs {
            self.compile_extern(e);
        }
    }

    fn compile_program(&mut self, funcs: Vec<hir::Function>) {
        for f in &funcs {
            self.declare_func(f);
        }
        for f in funcs {
            self.compile_func(f);
        }
    }

    fn compile_extern(&self, ext: hir::Extern) {
        let func_type = self.llvm_function_type(&ext.fun_ty);
        self.module.add_function(&ext.name, func_type, None);
    }

    fn declare_func(&self, f: &hir::Function) {
        let func_type = self.llvm_function_type(&f.fun_ty());
        self.module.add_function(&f.name, func_type, None);
    }

    fn compile_func(&mut self, f: hir::Function) {
        let function = self.get_llvm_func(&f.name);
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let mut ctx = CodeGenContext {
            function,
            lvars: Default::default(),
        };

        for stmt in &f.body_stmts {
            self.compile_expr(&mut ctx, stmt);
        }
    }

    fn compile_value_expr(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &hir::TypedExpr,
    ) -> inkwell::values::BasicValueEnum<'run> {
        match self.compile_expr(ctx, texpr) {
            Some(v) => v,
            None => panic!("this expression does not have value"),
        }
    }

    fn compile_expr(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &hir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        match &texpr.0 {
            hir::Expr::Number(n) => self.compile_number(*n),
            hir::Expr::PseudoVar(pvar) => self.compile_pseudo_var(pvar),
            hir::Expr::LVarRef(name) => self.compile_lvarref(ctx, name),
            hir::Expr::ArgRef(idx) => self.compile_argref(ctx, idx),
            hir::Expr::FuncRef(name) => self.compile_funcref(name),
            //            hir::Expr::OpCall(op, lhs, rhs) => {
            //                self.compile_op_call(blocks, block, lvars, op, lhs, rhs)
            //            }
            hir::Expr::FunCall(fexpr, arg_exprs) => self.compile_funcall(ctx, fexpr, arg_exprs),
            hir::Expr::If(cond, then, els) => self.compile_if(ctx, cond, then, els),
            //            hir::Expr::While(cond, exprs) => self.compile_while(blocks, block, lvars, cond, exprs),
            hir::Expr::Alloc(name) => self.compile_alloc(ctx, name),
            hir::Expr::Assign(name, rhs) => self.compile_assign(ctx, name, rhs),
            hir::Expr::Return(val_expr) => self.compile_return(ctx, val_expr),
            hir::Expr::Exprs(exprs) => self.compile_exprs(ctx, exprs),
            hir::Expr::Cast(expr, cast_type) => self.compile_cast(ctx, expr, cast_type),
            hir::Expr::Unbox(expr) => self.compile_unbox(ctx, expr),
            hir::Expr::RawI64(n) => self.compile_raw_i64(*n),
            //            hir::Expr::Br(expr, block_id) => self.compile_br(blocks, block, lvars, expr, block_id),
            //            hir::Expr::CondBr(cond, true_block_id, false_block_id) => {
            //                self.compile_cond_br(blocks, block, lvars, cond, true_block_id, false_block_id)
            //            }
            //            hir::Expr::BlockArgRef => self.compile_block_arg_ref(block),
            //            hir::Expr::Nop => Ok(None),
            _ => panic!("should be lowered before codegen.rs: {:?}", texpr.0),
        }
    }

    fn compile_number(&mut self, n: i64) -> Option<inkwell::values::BasicValueEnum<'run>> {
        Some(intrinsics::box_int(self, n).into())
    }

    fn compile_argref(
        &self,
        ctx: &mut CodeGenContext<'run>,
        idx: &usize,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = ctx.function.get_nth_param(*idx as u32).unwrap();
        Some(v)
    }

    fn compile_funcref(&self, name: &str) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let f = self
            .get_llvm_func(name)
            .as_global_value()
            .as_pointer_value();
        Some(f.into())
    }

    fn compile_pseudo_var(
        &mut self,
        pseudo_var: &hir::PseudoVar,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = match pseudo_var {
            hir::PseudoVar::True => intrinsics::box_bool(self, true),
            hir::PseudoVar::False => intrinsics::box_bool(self, false),
            // TODO: should be instance of Void
            hir::PseudoVar::Void => intrinsics::box_bool(self, false),
        };
        Some(v.into())
    }

    fn compile_lvarref(
        &self,
        ctx: &mut CodeGenContext<'run>,
        name: &str,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = ctx.lvars.get(name).unwrap();
        let t = self.ptr_type();
        let v = self.builder.build_load(t, *v, name).into_pointer_value();
        Some(SkObj(v).into())
    }

    fn compile_funcall(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        fexpr: &hir::TypedExpr,
        arg_exprs: &[hir::TypedExpr],
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let func = self.compile_value_expr(ctx, fexpr);
        let func_type = self.llvm_function_type(fexpr.1.as_fun_ty());
        let args = arg_exprs
            .iter()
            .map(|x| self.compile_value_expr(ctx, x).into())
            .collect::<Vec<_>>();
        let ret = self.builder.build_indirect_call(
            func_type,
            func.into_pointer_value(),
            &args,
            "calltmp",
        );
        Some(ret.try_as_basic_value().left().unwrap())
    }

    fn compile_if(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cond_expr: &hir::TypedExpr,
        then_exprs: &hir::TypedExpr,
        else_exprs: &Option<Box<hir::TypedExpr>>,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let begin_block = self.context.append_basic_block(ctx.function, "IfBegin");
        let then_block = self.context.append_basic_block(ctx.function, "IfThen");
        let else_block = self.context.append_basic_block(ctx.function, "IfElse");
        let merge_block = self.context.append_basic_block(ctx.function, "IfEnd");

        // IfBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        let cond_value = self.compile_value_expr(ctx, cond_expr);
        self.gen_conditional_branch(cond_value, then_block, else_block);
        // IfThen:
        self.builder.position_at_end(then_block);
        let then_value = self.compile_expr(ctx, then_exprs);
        if then_value.is_some() {
            self.builder.build_unconditional_branch(merge_block);
        }
        let then_block_end = self.builder.get_insert_block().unwrap();
        // IfElse:
        self.builder.position_at_end(else_block);
        let else_value = if let Some(else_exprs) = else_exprs {
            self.compile_expr(ctx, else_exprs)
        } else {
            None
        };
        if else_value.is_some() {
            self.builder.build_unconditional_branch(merge_block);
        }
        let else_block_end = self.builder.get_insert_block().unwrap();

        // IfEnd:
        self.builder.position_at_end(merge_block);
        match (then_value, else_value) {
            (None, None) => {
                self.builder.build_unreachable();
                None
            }
            (None, else_value) => else_value,
            (then_value, None) => then_value,
            (Some(then_val), Some(else_val)) => {
                let phi_node = self.builder.build_phi(self.ptr_type(), "ifResult");
                phi_node.add_incoming(&[(&then_val, then_block_end), (&else_val, else_block_end)]);
                Some(phi_node.as_basic_value())
            }
        }
    }

    fn compile_alloc(
        &self,
        ctx: &mut CodeGenContext<'run>,
        name: &str,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = self.builder.build_alloca(self.ptr_type(), name);
        ctx.lvars.insert(name.to_string(), v);
        None
    }

    fn compile_assign(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        name: &str,
        rhs: &hir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = self.compile_value_expr(ctx, rhs);
        let ptr = ctx.lvars.get(name).unwrap();
        self.builder.build_store(ptr.clone(), v.clone());
        Some(v)
    }

    fn compile_return(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        val_expr: &hir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let val = self.compile_value_expr(ctx, val_expr);
        self.builder.build_return(Some(&val));
        None
    }

    fn compile_exprs(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        exprs: &[hir::TypedExpr],
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let mut last_val = None;
        for e in exprs {
            last_val = self.compile_expr(ctx, e);
        }
        last_val
    }

    // TODO: just remove this?
    fn compile_cast<'a>(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        _cast_type: &hir::CastType,
        expr: &hir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let e = self.compile_value_expr(ctx, expr);
        //        let v = match cast_type {
        //            hir::CastType::AnyToFun(_) => e,
        //            hir::CastType::AnyToInt | hir::CastType::IntToAny | hir::CastType::VoidToAny => e,
        //            hir::CastType::FunToAny => e,
        //        };
        Some(e)
    }

    fn compile_unbox(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        expr: &hir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let e = self.compile_value_expr(ctx, expr);
        let sk_int = SkObj::from_basic_value_enum(e);
        Some(intrinsics::unbox_int(self, sk_int).into())
    }

    fn compile_raw_i64(&mut self, n: i64) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let llvm_n = self.context.i64_type().const_int(n as u64, false);
        Some(llvm_n.into())
    }

    /// Generate conditional branch by Shiika Bool
    fn gen_conditional_branch(
        &mut self,
        cond: inkwell::values::BasicValueEnum<'run>,
        then_block: inkwell::basic_block::BasicBlock,
        else_block: inkwell::basic_block::BasicBlock,
    ) {
        let i = intrinsics::unbox_bool(self, SkObj::from_basic_value_enum(cond));
        let one = self.context.bool_type().const_int(1, false);
        let istrue = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, i, one, "istrue");
        self.builder
            .build_conditional_branch(istrue, then_block, else_block);
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
            hir::Ty::Never => panic!("Never is unexpected here"),
            hir::Ty::Any => self.ptr_type().into(),
            hir::Ty::ChiikaEnv | hir::Ty::RustFuture => self.ptr_type().into(),
            hir::Ty::Bool | hir::Ty::Int | hir::Ty::Void => self.ptr_type().into(),
            hir::Ty::Fun(_) => self.ptr_type().into(),
            hir::Ty::Int64 => self.context.i64_type().into(),
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

    fn get_llvm_func(&self, name: &str) -> inkwell::values::FunctionValue<'run> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("function `{:?}' not found", name))
    }
}
