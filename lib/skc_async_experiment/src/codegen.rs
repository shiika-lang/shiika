use crate::names::FunctionName;
mod codegen_context;
mod instance;
mod intrinsics;
mod llvm_struct;
mod mir_analysis;
mod value;
use crate::mir;
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

pub fn run<P: AsRef<Path>>(bc_path: P, opt_ll_path: Option<P>, prog: mir::Program) -> Result<()> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let mut c = CodeGen {
        context: &context,
        module: &module,
        builder: &builder,
    };
    c.compile_externs(prog.externs);
    c.declare_const_globals(mir_analysis::list_constants::run(&prog.funcs));
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
    fn compile_externs(&mut self, externs: Vec<mir::Extern>) {
        for e in externs {
            self.compile_extern(e);
        }
    }

    fn compile_program(&mut self, funcs: Vec<mir::Function>) {
        for f in &funcs {
            self.declare_func(f);
        }
        for f in funcs {
            self.compile_func(f);
        }
    }

    fn compile_extern(&self, ext: mir::Extern) {
        let func_type = self.llvm_function_type(&ext.fun_ty);
        self.module
            .add_function(&ext.name.mangle(), func_type, None);
    }

    fn declare_func(&self, f: &mir::Function) {
        let func_type = self.llvm_function_type(&f.fun_ty());
        self.module.add_function(&f.name.mangle(), func_type, None);
    }

    fn declare_const_globals(&self, mut consts: Vec<(String, mir::Ty)>) {
        consts.push(("::Main".to_string(), mir::Ty::Raw("Meta:Main".to_string())));
        for (name, ty) in consts {
            debug_assert!(matches!(ty, mir::Ty::Raw(_)));
            let global = self.module.add_global(self.ptr_type(), None, &name);
            global.set_initializer(&self.ptr_type().const_null());
        }
    }

    fn compile_func(&mut self, f: mir::Function) {
        let function = self.get_llvm_func(&f.name);
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let mut ctx = CodeGenContext {
            function,
            lvars: Default::default(),
        };

        self.compile_expr(&mut ctx, &f.body_stmts);
    }

    fn compile_value_expr(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &mir::TypedExpr,
    ) -> inkwell::values::BasicValueEnum<'run> {
        match self.compile_expr(ctx, texpr) {
            Some(v) => v,
            None => panic!("this expression does not have value"),
        }
    }

    fn compile_expr(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        match &texpr.0 {
            mir::Expr::Number(n) => self.compile_number(*n),
            mir::Expr::PseudoVar(pvar) => Some(self.compile_pseudo_var(pvar)),
            mir::Expr::LVarRef(name) => self.compile_lvarref(ctx, name),
            mir::Expr::ArgRef(idx, _) => self.compile_argref(ctx, idx),
            mir::Expr::ConstRef(name) => self.compile_constref(name),
            mir::Expr::FuncRef(name) => self.compile_funcref(name),
            mir::Expr::FunCall(fexpr, arg_exprs) => self.compile_funcall(ctx, fexpr, arg_exprs),
            mir::Expr::If(cond, then, els) => self.compile_if(ctx, cond, then, els),
            mir::Expr::While(cond, exprs) => self.compile_while(ctx, cond, exprs),
            mir::Expr::Alloc(name) => self.compile_alloc(ctx, name),
            mir::Expr::Assign(name, rhs) => self.compile_assign(ctx, name, rhs),
            mir::Expr::ConstSet(name, rhs) => self.compile_const_set(ctx, name, rhs),
            mir::Expr::Return(val_expr) => self.compile_return(ctx, val_expr),
            mir::Expr::Exprs(exprs) => self.compile_exprs(ctx, exprs),
            mir::Expr::Cast(_, expr) => self.compile_cast(ctx, expr),
            mir::Expr::Unbox(expr) => self.compile_unbox(ctx, expr),
            mir::Expr::RawI64(n) => self.compile_raw_i64(*n),
            mir::Expr::Nop => None,
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

    fn compile_constref(&self, name: &str) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let g = self
            .module
            .get_global(name)
            .unwrap_or_else(|| panic!("global variable `{:?}' not found", name));
        let v = self
            .builder
            .build_load(self.ptr_type(), g.as_pointer_value(), name);
        Some(v.into())
    }

    fn compile_funcref(
        &self,
        name: &FunctionName,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let f = self
            .get_llvm_func(&name)
            .as_global_value()
            .as_pointer_value();
        Some(f.into())
    }

    fn compile_pseudo_var(
        &mut self,
        pseudo_var: &mir::PseudoVar,
    ) -> inkwell::values::BasicValueEnum<'run> {
        match pseudo_var {
            mir::PseudoVar::True => intrinsics::box_bool(self, true).into(),
            mir::PseudoVar::False => intrinsics::box_bool(self, false).into(),
            mir::PseudoVar::SelfRef => {
                // TODO: impl. self
                intrinsics::box_bool(self, true).into()
            }
            mir::PseudoVar::Void => return self.compile_void(),
        }
    }

    fn compile_void(&mut self) -> inkwell::values::BasicValueEnum<'run> {
        // TODO: should be instance of Void
        intrinsics::box_bool(self, false).into()
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
        fexpr: &mir::TypedExpr,
        arg_exprs: &[mir::TypedExpr],
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

    /// Compile a sync if
    fn compile_if(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cond_expr: &mir::TypedExpr,
        then_exprs: &mir::TypedExpr,
        else_exprs: &mir::TypedExpr,
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
        let else_value = self.compile_expr(ctx, else_exprs);
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

    /// Compile a sync while
    fn compile_while(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cond_expr: &mir::TypedExpr,
        body_expr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let cond_block = self.context.append_basic_block(ctx.function, "WhileCond");
        let body_block = self.context.append_basic_block(ctx.function, "WhileBody");
        let end_block = self.context.append_basic_block(ctx.function, "WhileEnd");

        // WhileCond:
        self.builder.build_unconditional_branch(cond_block);
        self.builder.position_at_end(cond_block);
        let cond_value = self.compile_value_expr(ctx, cond_expr);
        self.gen_conditional_branch(cond_value, body_block, end_block);

        // WhileBody:
        self.builder.position_at_end(body_block);
        self.compile_expr(ctx, body_expr);
        self.builder.build_unconditional_branch(cond_block);

        // WhileEnd:
        self.builder.position_at_end(end_block);
        Some(self.compile_void())
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
        rhs: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = self.compile_value_expr(ctx, rhs);
        let ptr = ctx.lvars.get(name).unwrap();
        self.builder.build_store(ptr.clone(), v.clone());
        Some(v)
    }

    fn compile_const_set(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        name: &str,
        rhs: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = self.compile_value_expr(ctx, rhs);
        let g = self.module.get_global(name).unwrap();
        self.builder.build_store(g.as_pointer_value(), v);
        Some(v)
    }

    fn compile_return(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        val_expr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let val = self.compile_value_expr(ctx, val_expr);
        self.builder.build_return(Some(&val));
        None
    }

    fn compile_exprs(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        exprs: &[mir::TypedExpr],
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let mut last_val = None;
        for e in exprs {
            last_val = self.compile_expr(ctx, e);
        }
        last_val
    }

    fn compile_cast<'a>(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        expr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let e = self.compile_value_expr(ctx, expr);
        Some(e)
    }

    fn compile_unbox(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        expr: &mir::TypedExpr,
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

    fn llvm_function_type(&self, fun_ty: &mir::FunTy) -> inkwell::types::FunctionType<'ictx> {
        let param_tys = self.llvm_types(&fun_ty.param_tys);
        let ret_ty = self.llvm_type(&fun_ty.ret_ty);
        ret_ty.fn_type(&param_tys, false)
    }

    fn llvm_types(&self, tys: &[mir::Ty]) -> Vec<inkwell::types::BasicMetadataTypeEnum<'ictx>> {
        tys.iter().map(|x| self.llvm_type(x).into()).collect()
    }

    fn llvm_type(&self, ty: &mir::Ty) -> inkwell::types::BasicTypeEnum<'ictx> {
        match ty {
            mir::Ty::Any => self.ptr_type().into(),
            mir::Ty::ChiikaEnv | mir::Ty::RustFuture => self.ptr_type().into(),
            mir::Ty::Fun(_) => self.ptr_type().into(),
            mir::Ty::Int64 => self.context.i64_type().into(),
            mir::Ty::Raw(s) => match s.as_str() {
                "Never" => panic!("Never is unexpected here"),
                _ => self.ptr_type().into(),
            },
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
        self.module
            .get_function(&name.mangle())
            .unwrap_or_else(|| panic!("function `{:?}' not found", name))
    }
}
