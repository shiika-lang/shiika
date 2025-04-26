use crate::mir;
use anyhow::Result;

pub trait MirVisitor {
    /// Callback function.
    fn visit_expr(&mut self, expr: &mir::TypedExpr) -> Result<()>;

    //fn walk_mir(&mut self, mir: &mir::Program) -> Result<()> {
    //    self.walk_funcs(&mir.funcs)?;
    //    Ok(())
    //}

    //fn walk_funcs(&mut self, funcs: &[mir::Function]) -> Result<()> {
    //    for f in funcs {
    //        self.walk_expr(&f.body_stmts)?;
    //    }
    //    Ok(())
    //}

    fn walk_exprs(&mut self, exprs: &[mir::TypedExpr]) -> Result<()> {
        for expr in exprs {
            self.walk_expr(expr)?;
        }
        Ok(())
    }

    fn walk_expr(&mut self, expr: &mir::TypedExpr) -> Result<()> {
        match &expr.0 {
            mir::Expr::Number(_) => {}
            mir::Expr::PseudoVar(_) => {}
            mir::Expr::LVarRef(_) => {}
            mir::Expr::ArgRef(_, _) => {}
            mir::Expr::EnvRef(_, _) => {}
            mir::Expr::EnvSet(_, value_expr, _) => {
                self.walk_expr(value_expr)?;
            }
            mir::Expr::ConstRef(_) => {}
            mir::Expr::FuncRef(_) => {}
            mir::Expr::FunCall(fexpr, arg_exprs) => {
                self.walk_expr(fexpr)?;
                for arg in arg_exprs {
                    self.walk_expr(arg)?;
                }
            }
            mir::Expr::If(cond_expr, then_exprs, else_exprs) => {
                self.walk_expr(cond_expr)?;
                self.walk_expr(then_exprs)?;
                self.walk_expr(else_exprs)?;
            }
            mir::Expr::While(cond_expr, body_exprs) => {
                self.walk_expr(cond_expr)?;
                self.walk_expr(body_exprs)?;
            }
            mir::Expr::Spawn(expr) => {
                self.walk_expr(expr)?;
            }
            mir::Expr::Alloc(_, _) => {}
            mir::Expr::Assign(_, rhs) => {
                self.walk_expr(rhs)?;
            }
            mir::Expr::ConstSet(_, rhs) => {
                self.walk_expr(rhs)?;
            }
            mir::Expr::Return(expr) => {
                self.walk_expr(expr)?;
            }
            mir::Expr::Exprs(exprs) => {
                self.walk_exprs(exprs)?;
            }
            mir::Expr::Cast(_, expr) => {
                self.walk_expr(expr)?;
            }
            mir::Expr::CreateObject(_) => {}
            mir::Expr::CreateTypeObject(_) => {}
            mir::Expr::Unbox(expr) => {
                self.walk_expr(expr)?;
            }
            mir::Expr::RawI64(_) => {}
            mir::Expr::Nop => {}
        }
        self.visit_expr(expr)?;
        Ok(())
    }
}

pub struct Allocs(Vec<(String, mir::Ty)>);
impl Allocs {
    /// Collects `alloc`ed variable names and their types.
    pub fn collect(body_stmts: &mir::TypedExpr) -> Vec<(String, mir::Ty)> {
        let mut a = Allocs(vec![]);
        a.walk_expr(body_stmts).unwrap();
        a.0
    }
}
impl MirVisitor for Allocs {
    fn visit_expr(&mut self, texpr: &mir::TypedExpr) -> Result<()> {
        match texpr {
            (mir::Expr::Alloc(name, ty), _) => {
                self.0.push((name.clone(), ty.clone()));
            }
            _ => {}
        }
        Ok(())
    }
}
