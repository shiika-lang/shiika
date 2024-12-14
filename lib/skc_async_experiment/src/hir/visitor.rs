use crate::hir;
use anyhow::Result;

pub trait HirVisitor {
    /// Callback function.
    fn visit_expr(&mut self, expr: &hir::TypedExpr) -> Result<()>;

    fn walk_hir(&mut self, hir: &hir::Program) -> Result<()> {
        for f in &hir.funcs {
            self.walk_expr(&f.body_stmts)?;
        }
        Ok(())
    }

    fn walk_exprs(&mut self, exprs: &[hir::TypedExpr]) -> Result<()> {
        for expr in exprs {
            self.walk_expr(expr)?;
        }
        Ok(())
    }

    fn walk_expr(&mut self, expr: &hir::TypedExpr) -> Result<()> {
        match &expr.0 {
            hir::Expr::Number(_) => {}
            hir::Expr::PseudoVar(_) => {}
            hir::Expr::LVarRef(_) => {}
            hir::Expr::ArgRef(_, _) => {}
            hir::Expr::FuncRef(_) => {}
            hir::Expr::FunCall(fexpr, arg_exprs) => {
                self.walk_expr(fexpr)?;
                for arg in arg_exprs {
                    self.walk_expr(arg)?;
                }
            }
            hir::Expr::If(cond_expr, then_exprs, else_exprs) => {
                self.walk_expr(cond_expr)?;
                self.walk_expr(then_exprs)?;
                self.walk_expr(else_exprs)?;
            }
            hir::Expr::While(cond_expr, body_exprs) => {
                self.walk_expr(cond_expr)?;
                for expr in body_exprs {
                    self.walk_expr(expr)?;
                }
            }
            hir::Expr::Spawn(expr) => {
                self.walk_expr(expr)?;
            }
            hir::Expr::Alloc(_) => {}
            hir::Expr::Assign(_, rhs) => {
                self.walk_expr(rhs)?;
            }
            hir::Expr::Return(expr) => {
                self.walk_expr(expr)?;
            }
            hir::Expr::Exprs(exprs) => {
                self.walk_exprs(exprs)?;
            }
            hir::Expr::Cast(_, expr) => {
                self.walk_expr(expr)?;
            }
            _ => todo!("{:?}", expr),
        }
        self.visit_expr(expr)?;
        Ok(())
    }
}

pub struct Allocs(Vec<(String, hir::Ty)>);
impl Allocs {
    /// Collects `alloc`ed variable names and their types.
    pub fn collect(body_stmts: &hir::TypedExpr) -> Result<Vec<(String, hir::Ty)>> {
        let mut a = Allocs(vec![]);
        a.walk_expr(body_stmts)?;
        Ok(a.0)
    }
}
impl HirVisitor for Allocs {
    fn visit_expr(&mut self, texpr: &hir::TypedExpr) -> Result<()> {
        match texpr {
            (hir::Expr::Alloc(name), ty) => {
                self.0.push((name.clone(), ty.clone()));
            }
            _ => {}
        }
        Ok(())
    }
}

//pub struct NoAsyncTy(());
//impl NoAsyncTy {
//    /// Asserts that there is no `async` type in the program.
//    pub fn check(hir: &hir::Program) -> Result<()> {
//        let mut a = NoAsyncTy(());
//        for e in &hir.externs {
//            for p in &e.params {
//                assert_no_async_ty(&p.ty).context(format!("in extern: {:?}", e))?;
//            }
//            assert_no_async_ty(&e.ret_ty).context(format!("in extern: {:?}", e))?;
//        }
//        for f in &hir.funcs {
//            for p in &f.params {
//                assert_no_async_ty(&p.ty).context(format!("in func: {:?}", f))?;
//            }
//            assert_no_async_ty(&f.ret_ty).context(format!("in func: {:?}", f))?;
//        }
//        a.walk_hir(hir)?;
//        Ok(a.0)
//    }
//}
//impl HirVisitor for NoAsyncTy {
//    fn visit_expr(&mut self, texpr: &hir::TypedExpr) -> Result<()> {
//        assert_no_async_ty(&texpr.1).context(format!("in expr: {:?}", texpr))
//    }
//}
//
//fn assert_no_async_ty(ty: &hir::Ty) -> Result<()> {
//    if matches!(ty, hir::Ty::Async(_)) {
//        Err(anyhow::anyhow!("async type found: {:?}", ty))
//    } else {
//        Ok(())
//    }
//}
