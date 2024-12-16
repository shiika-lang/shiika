use crate::hir;
use anyhow::Result;

pub trait HirRewriter {
    /// Callback function.
    fn rewrite_expr(&mut self, expr: hir::TypedExpr) -> Result<hir::TypedExpr>;

    fn walk_hir(&mut self, hir: hir::Program) -> Result<hir::Program> {
        let funcs = hir
            .funcs
            .into_iter()
            .map(|f| {
                let body_stmts = self.walk_expr(f.body_stmts)?;
                Ok(hir::Function { body_stmts, ..f })
            })
            .collect::<Result<_>>()?;
        Ok(hir::Program { funcs, ..hir })
    }

    fn walk_exprs(&mut self, exprs: Vec<hir::TypedExpr>) -> Result<Vec<hir::TypedExpr>> {
        exprs.into_iter().map(|expr| self.walk_expr(expr)).collect()
    }

    fn walk_expr(&mut self, expr: hir::TypedExpr) -> Result<hir::TypedExpr> {
        let new_expr = match expr.0 {
            hir::Expr::Number(_) => expr,
            hir::Expr::PseudoVar(_) => expr,
            hir::Expr::LVarRef(_) => expr,
            hir::Expr::ArgRef(_, _) => expr,
            hir::Expr::EnvRef(_, _) => expr,
            hir::Expr::EnvSet(idx, value_expr, name) => {
                hir::Expr::env_set(idx, self.walk_expr(*value_expr)?, name)
            }
            hir::Expr::FuncRef(_) => expr,
            hir::Expr::FunCall(fexpr, arg_exprs) => {
                hir::Expr::fun_call(self.walk_expr(*fexpr)?, self.walk_exprs(arg_exprs)?)
            }
            hir::Expr::If(cond_expr, then_exprs, else_exprs) => hir::Expr::if_(
                self.walk_expr(*cond_expr)?,
                self.walk_expr(*then_exprs)?,
                self.walk_expr(*else_exprs)?,
            ),
            hir::Expr::While(cond_expr, body_exprs) => {
                hir::Expr::while_(self.walk_expr(*cond_expr)?, self.walk_exprs(body_exprs)?)
            }
            hir::Expr::Spawn(expr) => hir::Expr::spawn(self.walk_expr(*expr)?),
            hir::Expr::Alloc(_) => expr,
            hir::Expr::Assign(name, rhs) => hir::Expr::assign(name, self.walk_expr(*rhs)?),
            hir::Expr::Return(expr) => hir::Expr::return_(self.walk_expr(*expr)?),
            hir::Expr::Exprs(exprs) => hir::Expr::exprs(self.walk_exprs(exprs)?),
            hir::Expr::Cast(cast_type, expr) => hir::Expr::cast(cast_type, self.walk_expr(*expr)?),
            hir::Expr::RawI64(_) => expr,
            _ => panic!("not supported by hir::rewriter: {:?}", expr),
        };
        self.rewrite_expr(new_expr)
    }
}
