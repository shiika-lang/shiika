use crate::hir;
use anyhow::Result;

pub trait HirVisitor<T> {
    /// Callback function.
    fn visit_expr(&mut self, expr: &hir::TypedExpr<T>) -> Result<()>;

    fn walk_exprs(&mut self, exprs: &[hir::TypedExpr<T>]) -> Result<()> {
        for expr in exprs {
            self.walk_expr(expr)?;
        }
        Ok(())
    }

    fn walk_expr(&mut self, expr: &hir::TypedExpr<T>) -> Result<()> {
        match &expr.0 {
            hir::Expr::Number(_) => {}
            hir::Expr::PseudoVar(_) => {}
            hir::Expr::LVarRef(_) => {}
            hir::Expr::ArgRef(_, _) => {}
            hir::Expr::ConstRef(_) => {}
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
            hir::Expr::MethodCall(receiver_expr, _, arg_exprs) => {
                self.walk_expr(receiver_expr)?;
                for arg in arg_exprs {
                    self.walk_expr(arg)?;
                }
            }
            hir::Expr::While(cond_expr, body_exprs) => {
                self.walk_expr(cond_expr)?;
                self.walk_expr(body_exprs)?;
            }
            hir::Expr::Spawn(expr) => {
                self.walk_expr(expr)?;
            }
            hir::Expr::LVarDecl(_, rhs) => {
                self.walk_expr(rhs)?;
            }
            hir::Expr::Assign(_, rhs) => {
                self.walk_expr(rhs)?;
            }
            hir::Expr::ConstSet(_, rhs) => {
                self.walk_expr(rhs)?;
            }
            hir::Expr::Return(expr) => {
                self.walk_expr(expr)?;
            }
            hir::Expr::Exprs(exprs) => {
                self.walk_exprs(exprs)?;
            }
            hir::Expr::Upcast(expr, _) => {
                self.walk_expr(expr)?;
            }
            hir::Expr::CreateObject(_) => {}
            hir::Expr::CreateTypeObject(_) => {}
        }
        self.visit_expr(expr)?;
        Ok(())
    }
}
