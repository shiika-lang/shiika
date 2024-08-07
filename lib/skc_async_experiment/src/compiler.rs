use crate::hir;
use anyhow::{anyhow, Result};
use melior::{
    dialect::{
        self,
        ods,
        //ods::r#async,
        DialectRegistry,
    },
    ir::{
        self,
        attribute::{FlatSymbolRefAttribute, IntegerAttribute, StringAttribute, TypeAttribute},
        r#type::{FunctionType, MemRefType, Type},
    },
    //pass::{self, PassManager},
    utility::{register_all_dialects, register_all_llvm_translations},
};
use train_map::TrainMap;

/// Get the first result value of an operation.
/// Panics if the operation yields no value
fn val<'c, 'a>(x: ir::OperationRef<'c, 'a>) -> ir::Value<'c, 'a> {
    x.result(0)
        .unwrap_or_else(|_| panic!("this operation has no value: {x}"))
        .into()
}

//fn vals<'c>(xs: &'c [ir::OperationRef<'c, 'a>]) -> Vec<ir::Value<'c, 'a>> {
//    let mut v = vec![];
//    for x in xs {
//        v.push(val(x));
//    }
//    v
//}

struct Compiler<'c> {
    //filename: &'c str,
    //src: &'c str,
    context: &'c melior::Context,
}

pub fn run(_filename: &str, _src: &str, prog: hir::blocked::Program) -> Result<()> {
    let registry = DialectRegistry::new();
    register_all_dialects(&registry);

    let context = melior::Context::new();
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
    register_all_llvm_translations(&context);

    let c = Compiler {
        //filename,
        //src,
        context: &context,
    };
    c.compile_program(prog)?;
    Ok(())
}

impl<'c> Compiler<'c> {
    fn compile_program(&self, prog: hir::blocked::Program) -> Result<()> {
        let module = ir::Module::new(self.unknown_loc());
        let block = module.body();

        for e in prog.externs {
            block.append_operation(self.compile_extern(e)?);
        }
        for f in prog.funcs {
            block.append_operation(self.compile_func(f)?);
        }

        //module.as_operation().dump();
        //println!("--");
        //assert!(module.as_operation().verify());

        // Convert to LLVM Dialect
        //let pass_manager = PassManager::new(&self.context);
        //pass_manager.add_pass(pass::r#async::create_async_func_to_async_runtime());
        //pass_manager.add_pass(pass::r#async::create_async_to_async_runtime());
        //pass_manager.add_pass(pass::conversion::create_async_to_llvm());
        //pass_manager.add_pass(pass::conversion::create_func_to_llvm());
        //pass_manager
        //    .nested_under("func.func")
        //    .add_pass(pass::conversion::create_arith_to_llvm());
        //pass_manager
        //    .nested_under("func.func")
        //    .add_pass(pass::conversion::create_index_to_llvm());
        //pass_manager.add_pass(pass::conversion::create_scf_to_control_flow());
        //pass_manager.add_pass(pass::conversion::create_control_flow_to_llvm());
        //pass_manager.add_pass(pass::conversion::create_finalize_mem_ref_to_llvm());
        //pass_manager.run(&mut module).unwrap();
        module.as_operation().dump();
        //assert!(module.as_operation().verify());
        Ok(())
    }

    fn compile_extern(&self, ext: hir::Extern) -> Result<ir::Operation> {
        let attrs = vec![(
            self.identifier("sym_visibility"),
            self.str_attr("private").into(),
        )];
        Ok(dialect::func::func(
            &self.context,
            self.str_attr(&ext.name),
            TypeAttribute::new(self.function_type(&ext.fun_ty())?.into()),
            Default::default(),
            &attrs,
            self.unknown_loc(),
        ))
    }

    /// Entry point.
    fn compile_func(&self, f: hir::blocked::Function) -> Result<ir::Operation<'c>> {
        let region = ir::Region::new();
        let mut lvars = TrainMap::new();

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

    fn compile_value_expr<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        expr: &hir::TypedExpr,
    ) -> Result<ir::Value<'c, 'a>> {
        match self.compile_expr(blocks, block, lvars, expr)? {
            Some(v) => Ok(v),
            None => Err(anyhow!("this expression does not have value")),
        }
    }

    fn compile_expr<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        texpr: &hir::TypedExpr,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        match &texpr.0 {
            hir::Expr::Number(n) => self.compile_number(block, *n),
            hir::Expr::PseudoVar(pvar) => self.compile_pseudo_var(block, pvar),
            hir::Expr::LVarRef(name) => self.compile_lvarref(block, lvars, name),
            hir::Expr::ArgRef(idx) => self.compile_argref(blocks, idx),
            hir::Expr::FuncRef(name) => {
                let hir::Ty::Fun(fun_ty) = &texpr.1 else {
                    return Err(anyhow!("[BUG] not a function: {:?}", texpr.1));
                };
                self.compile_funcref(block, name, &fun_ty)
            }
            hir::Expr::OpCall(op, lhs, rhs) => {
                self.compile_op_call(blocks, block, lvars, op, lhs, rhs)
            }
            hir::Expr::FunCall(fexpr, arg_exprs) => {
                self.compile_funcall(blocks, block, lvars, fexpr, arg_exprs)
            }
            hir::Expr::If(cond, then, els) => {
                self.compile_if(blocks, block, lvars, cond, then, els, &texpr.1)
            }
            hir::Expr::Yield(expr) => self.compile_yield(blocks, block, lvars, expr),
            hir::Expr::While(cond, exprs) => self.compile_while(blocks, block, lvars, cond, exprs),
            hir::Expr::Alloc(name) => self.compile_alloc(block, lvars, name),
            hir::Expr::Assign(name, rhs) => self.compile_assign(blocks, block, lvars, name, rhs),
            hir::Expr::Return(val_expr) => self.compile_return(blocks, block, lvars, val_expr),
            hir::Expr::Cast(expr, cast_type) => {
                self.compile_cast(blocks, block, lvars, expr, cast_type)
            }
            hir::Expr::Br(expr, block_id) => self.compile_br(blocks, block, lvars, expr, block_id),
            hir::Expr::CondBr(cond, true_block_id, false_block_id) => {
                self.compile_cond_br(blocks, block, lvars, cond, true_block_id, false_block_id)
            }
            hir::Expr::BlockArgRef => self.compile_block_arg_ref(block),
            hir::Expr::Nop => Ok(None),
            _ => panic!("should be lowered before compiler.rs: {:?}", texpr.0),
        }
    }

    fn compile_op_call<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        operator: &str,
        lhs: &hir::TypedExpr,
        rhs: &hir::TypedExpr,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let f = match operator {
            "+" => dialect::arith::addi,
            "-" => dialect::arith::subi,
            "*" => dialect::arith::muli,
            "/" => dialect::arith::divsi,
            _ => return self.compile_cmp(blocks, block, lvars, operator, lhs, rhs),
        };
        let op = f(
            self.compile_value_expr(blocks, block, lvars, lhs)?,
            self.compile_value_expr(blocks, block, lvars, rhs)?,
            self.unknown_loc(),
        );
        Ok(Some(val(block.append_operation(op))))
    }

    fn compile_cmp<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        operator: &str,
        lhs: &hir::TypedExpr,
        rhs: &hir::TypedExpr,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let pred = match operator {
            "==" => dialect::arith::CmpiPredicate::Eq,
            "!=" => dialect::arith::CmpiPredicate::Ne,
            "<" => dialect::arith::CmpiPredicate::Ult,
            "<=" => dialect::arith::CmpiPredicate::Ule,
            ">" => dialect::arith::CmpiPredicate::Ugt,
            ">=" => dialect::arith::CmpiPredicate::Uge,
            _ => panic!("unkown operator"),
        };
        let op = dialect::arith::cmpi(
            &self.context,
            pred,
            self.compile_value_expr(blocks, block, lvars, lhs)?,
            self.compile_value_expr(blocks, block, lvars, rhs)?,
            self.unknown_loc(),
        );
        Ok(Some(val(block.append_operation(op))))
    }

    fn compile_funcall<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        fexpr: &hir::TypedExpr,
        arg_exprs: &[hir::TypedExpr],
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let hir::Ty::Fun(fun_ty) = &fexpr.1 else {
            return Err(anyhow!("[BUG] not a function: {:?}", fexpr.1));
        };

        let f = self.compile_value_expr(blocks, block, lvars, fexpr)?;

        let mut args = vec![];
        for e in arg_exprs {
            args.push(self.compile_value_expr(blocks, block, lvars, e)?.into());
        }

        let result_types = vec![self.mlir_type(&fun_ty.ret_ty)?];
        let op = dialect::func::call_indirect(f, &args, &result_types, self.unknown_loc());
        Ok(Some(val(block.append_operation(op))))
    }

    fn compile_if<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        cond_expr: &hir::TypedExpr,
        then: &[hir::TypedExpr],
        els: &[hir::TypedExpr],
        if_ty: &hir::Ty,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let cond_result = self.compile_value_expr(blocks, block, lvars, cond_expr)?;
        let then_region = {
            let region = ir::Region::new();
            region.append_block(self.compile_exprs(blocks, lvars, then)?);
            region
        };
        let else_region = {
            let region = ir::Region::new();
            region.append_block(self.compile_exprs(blocks, lvars, els)?);
            region
        };
        let op = dialect::scf::r#if(
            cond_result,
            &[self.mlir_type(if_ty)?],
            then_region,
            else_region,
            self.unknown_loc(),
        );
        Ok(Some(val(block.append_operation(op))))
    }

    fn compile_yield<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        expr: &hir::TypedExpr,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let v = self.compile_value_expr(blocks, block, lvars, expr)?;
        let op = dialect::scf::r#yield(&[v], self.unknown_loc());
        block.append_operation(op);
        Ok(None)
    }

    fn compile_while<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        cond_expr: &hir::TypedExpr,
        exprs: &[hir::TypedExpr],
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let before_region = {
            let region = ir::Region::new();
            let block = ir::Block::new(&[]);
            let mut lvars = lvars.fork();
            let v = self.compile_value_expr(blocks, &block, &mut lvars, cond_expr)?;
            block.append_operation(dialect::scf::condition(v, &[], self.unknown_loc()));
            region.append_block(block);
            region
        };
        let after_region = {
            let region = ir::Region::new();
            let block = self.compile_exprs(blocks, lvars, exprs)?;
            region.append_block(block);
            region
        };
        block.append_operation(dialect::scf::r#while(
            &[],
            &[],
            before_region,
            after_region,
            self.unknown_loc(),
        ));
        Ok(None)
    }

    fn compile_alloc<'a>(
        &self,
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        name: &str,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let op = dialect::memref::alloca(
            &self.context,
            MemRefType::new(self.int_type().into(), &[], None, None),
            &[],
            &[],
            None,
            self.unknown_loc(),
        );
        let v = val(block.append_operation(op));
        lvars.insert(name.to_string(), v.clone());
        Ok(Some(v))
    }

    fn compile_assign<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        name: &str,
        rhs: &hir::TypedExpr,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let rhs_result = self.compile_value_expr(blocks, block, lvars, rhs)?;
        let Some(lvar) = lvars.get(name) else {
            return Err(anyhow!("unknown lvar {name}"));
        };
        let op = dialect::memref::store(rhs_result, *lvar, &[], self.unknown_loc());
        block.append_operation(op);
        Ok(None)
    }

    fn compile_return<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        expr: &hir::TypedExpr,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let v = self.compile_value_expr(blocks, block, lvars, expr)?;
        let op = dialect::func::r#return(&[v], self.unknown_loc());
        block.append_operation(op);
        Ok(None)
    }

    fn compile_cast<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        cast_type: &hir::CastType,
        expr: &hir::TypedExpr,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let e = self.compile_value_expr(blocks, block, lvars, expr)?;
        let v = match cast_type {
            hir::CastType::AnyToFun(fun_ty) => {
                let op = ods::llvm::inttoptr(
                    self.context,
                    self.ptr_type().into(),
                    e,
                    self.unknown_loc(),
                );
                let v = val(block.append_operation(op.into()));
                let op = ods::builtin::unrealized_conversion_cast(
                    self.context,
                    &[self.function_type(fun_ty)?.into()],
                    &[v],
                    self.unknown_loc(),
                );
                val(block.append_operation(op.into()))
            }
            hir::CastType::AnyToInt | hir::CastType::IntToAny | hir::CastType::NullToAny => e,
            hir::CastType::FunToAny => {
                let op = ods::builtin::unrealized_conversion_cast(
                    self.context,
                    &[self.ptr_type().into()],
                    &[e],
                    self.unknown_loc(),
                );
                let v = val(block.append_operation(op.into()));
                let op = ods::llvm::ptrtoint(
                    self.context,
                    self.int_type().into(),
                    v,
                    self.unknown_loc(),
                );
                val(block.append_operation(op.into()))
            }
        };
        Ok(Some(v))
    }

    fn compile_lvarref<'a>(
        &self,
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        name: &str,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let Some(v) = lvars.get(name) else {
            return Err(anyhow!("[BUG] unknown variable `{name}'"));
        };
        let op = dialect::memref::load(v.clone(), &[], self.unknown_loc());
        Ok(Some(val(block.append_operation(op))))
    }

    fn compile_argref<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        idx: &usize,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        Ok(Some(blocks.first().unwrap().argument(*idx).unwrap().into()))
    }

    fn compile_funcref<'a>(
        &self,
        block: &'a ir::Block<'c>,
        name: &str,
        fun_ty: &hir::FunTy,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let op = dialect::func::constant(
            &self.context,
            FlatSymbolRefAttribute::new(self.context, name),
            self.function_type(fun_ty)?,
            self.unknown_loc(),
        );
        Ok(Some(val(block.append_operation(op))))
    }

    fn compile_number<'a>(
        &self,
        block: &'a ir::Block<'c>,
        n: i64,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        Ok(Some(val(block.append_operation(self.const_int(n)))))
    }

    fn compile_pseudo_var<'a>(
        &self,
        block: &'a ir::Block<'c>,
        pseudo_var: &hir::PseudoVar,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let op = match pseudo_var {
            hir::PseudoVar::True => dialect::arith::constant(
                &self.context,
                IntegerAttribute::new(self.bool_type().into(), 1).into(),
                self.unknown_loc(),
            ),
            hir::PseudoVar::False => dialect::arith::constant(
                &self.context,
                IntegerAttribute::new(self.bool_type().into(), 0).into(),
                self.unknown_loc(),
            ),
            // Null is represented as `i64 0`
            hir::PseudoVar::Null => dialect::arith::constant(
                &self.context,
                IntegerAttribute::new(self.int_type().into(), 0).into(),
                self.unknown_loc(),
            ),
        };
        Ok(Some(val(block.append_operation(op))))
    }

    fn compile_br<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        expr: &hir::TypedExpr,
        block_id: &usize,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let v = self.compile_value_expr(blocks, block, lvars, expr)?;
        let op = dialect::cf::br(&blocks[*block_id], &[v], self.unknown_loc());
        block.append_operation(op);
        Ok(None)
    }

    fn compile_cond_br<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        block: &'a ir::Block<'c>,
        lvars: &mut TrainMap<String, ir::Value<'c, 'a>>,
        cond_expr: &hir::TypedExpr,
        true_block_id: &usize,
        false_block_id: &usize,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        let v = self.compile_value_expr(blocks, block, lvars, cond_expr)?;
        let op = dialect::cf::cond_br(
            &self.context,
            v,
            &blocks[*true_block_id],
            &blocks[*false_block_id],
            &[],
            &[],
            self.unknown_loc(),
        );
        block.append_operation(op);
        Ok(None)
    }

    fn compile_block_arg_ref<'a>(
        &self,
        block: &'a ir::Block<'c>,
    ) -> Result<Option<ir::Value<'c, 'a>>> {
        Ok(Some(block.argument(0).unwrap().into()))
    }

    /// Returns a newly created region that contains `exprs`.
    fn compile_exprs<'a>(
        &self,
        blocks: &'a [ir::Block<'c>],
        lvars: &mut TrainMap<String, ir::Value<'c, '_>>,
        exprs: &[hir::TypedExpr],
    ) -> Result<ir::Block<'c>> {
        let block = ir::Block::new(&[]);
        let mut lvars = lvars.fork();
        for expr in exprs {
            self.compile_expr(blocks, &block, &mut lvars, expr)?;
        }
        Ok(block)
    }

    fn const_int(&self, n: i64) -> ir::Operation<'c> {
        dialect::arith::constant(
            &self.context,
            IntegerAttribute::new(self.int_type().into(), n).into(),
            self.unknown_loc(),
        )
    }

    fn function_type(&self, fun_ty: &hir::FunTy) -> Result<ir::r#type::FunctionType<'c>> {
        let param_tys = self.mlir_types(&fun_ty.param_tys)?;
        let ret_ty = self.mlir_type(&fun_ty.ret_ty)?;
        Ok(FunctionType::new(&self.context, &param_tys, &[ret_ty]).into())
    }

    fn mlir_types(&self, tys: &[hir::Ty]) -> Result<Vec<ir::Type<'c>>> {
        tys.iter().map(|x| self.mlir_type(x)).collect()
    }

    fn mlir_type(&self, ty: &hir::Ty) -> Result<ir::Type<'c>> {
        let t = match ty {
            hir::Ty::Unknown => return Err(anyhow!("Unknown is unexpected here")),
            hir::Ty::Void => return Err(anyhow!("void is unexpected here")),
            hir::Ty::ChiikaEnv | hir::Ty::RustFuture => self.ptr_type().into(),
            hir::Ty::Any | hir::Ty::Int | hir::Ty::Null => self.int_type().into(),
            hir::Ty::Bool => Type::parse(&self.context, "i1").unwrap(),
            hir::Ty::Fun(fun_ty) => self.function_type(fun_ty)?.into(),
        };
        Ok(t)
    }

    fn ptr_type(&self) -> ir::Type<'c> {
        Type::parse(&self.context, "!llvm.ptr").unwrap()
    }

    fn bool_type(&self) -> ir::Type<'c> {
        ir::r#type::IntegerType::new(&self.context, 1).into()
    }

    fn int_type(&self) -> ir::Type<'c> {
        ir::r#type::IntegerType::new(&self.context, 64).into()
    }

    fn identifier(&self, s: &str) -> ir::Identifier<'c> {
        ir::Identifier::new(&self.context, s)
    }

    fn str_attr(&self, s: &str) -> StringAttribute<'c> {
        StringAttribute::new(&self.context, s)
    }

    //fn loc(&self, span: &ast::Span) -> ir::Location<'c> {
    //    ir::Location::new(
    //        &self.context,
    //        &self.filename,
    //        span.location_line() as usize,
    //        span.get_utf8_column(),
    //    )
    //}

    fn unknown_loc(&self) -> ir::Location<'c> {
        ir::Location::unknown(&self.context)
    }
}
