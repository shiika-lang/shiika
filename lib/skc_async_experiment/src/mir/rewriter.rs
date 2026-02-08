use crate::mir;
use anyhow::Result;

pub trait MirRewriter {
    /// Callback function.
    fn rewrite_expr(&mut self, expr: mir::TypedExpr) -> Result<mir::TypedExpr>;

    fn walk_mir(&mut self, mir: mir::Program) -> Result<mir::Program> {
        let funcs = mir
            .funcs
            .into_iter()
            .map(|f| {
                let body_stmts = self.walk_expr(f.body_stmts)?;
                Ok(mir::Function { body_stmts, ..f })
            })
            .collect::<Result<_>>()?;
        Ok(mir::Program { funcs, ..mir })
    }

    fn walk_exprs(&mut self, exprs: Vec<mir::TypedExpr>) -> Result<Vec<mir::TypedExpr>> {
        exprs.into_iter().map(|expr| self.walk_expr(expr)).collect()
    }

    fn walk_expr(&mut self, expr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let new_expr = match expr.0 {
            mir::Expr::Number(_) => expr,
            mir::Expr::PseudoVar(_) => expr,
            mir::Expr::LVarRef(_) => expr,
            mir::Expr::IVarRef(obj_expr, idx, name) => {
                mir::Expr::ivar_ref(self.walk_expr(*obj_expr)?, idx, name, expr.1.clone())
            }
            mir::Expr::ArgRef(_, _) => expr,
            mir::Expr::EnvRef(_, _) => expr,
            mir::Expr::EnvSet(idx, value_expr, name) => {
                mir::Expr::env_set(idx, self.walk_expr(*value_expr)?, name)
            }
            mir::Expr::ConstRef(_) => expr,
            mir::Expr::FuncRef(_) => expr,
            mir::Expr::FunCall(fexpr, arg_exprs) => {
                mir::Expr::fun_call(self.walk_expr(*fexpr)?, self.walk_exprs(arg_exprs)?)
            }
            mir::Expr::VTableRef(receiver, idx, name) => {
                mir::Expr::vtable_ref(self.walk_expr(*receiver)?, idx, name, expr.1.into_fun_ty())
            }
            mir::Expr::WTableRef(receiver, module, idx, name) => mir::Expr::wtable_ref(
                self.walk_expr(*receiver)?,
                module,
                idx,
                name,
                expr.1.into_fun_ty(),
            ),
            mir::Expr::If(cond_expr, then_exprs, else_exprs) => mir::Expr::if_(
                self.walk_expr(*cond_expr)?,
                self.walk_expr(*then_exprs)?,
                self.walk_expr(*else_exprs)?,
            ),
            mir::Expr::While(cond_expr, body_exprs) => {
                mir::Expr::while_(self.walk_expr(*cond_expr)?, self.walk_expr(*body_exprs)?)
            }
            mir::Expr::Spawn(expr) => mir::Expr::spawn(self.walk_expr(*expr)?),
            mir::Expr::Alloc(_, _) => expr,
            mir::Expr::LVarDecl(name, rhs, writable) => {
                mir::Expr::lvar_decl(name, self.walk_expr(*rhs)?, writable)
            }
            mir::Expr::LVarSet(name, rhs) => mir::Expr::lvar_set(name, self.walk_expr(*rhs)?),
            mir::Expr::IVarSet(obj, idx, rhs, name) => {
                mir::Expr::ivar_set(self.walk_expr(*obj)?, idx, self.walk_expr(*rhs)?, name)
            }
            mir::Expr::ConstSet(name, rhs) => mir::Expr::const_set(name, self.walk_expr(*rhs)?),
            mir::Expr::Return(expr) => {
                if let Some(e) = expr {
                    mir::Expr::return_(self.walk_expr(*e)?)
                } else {
                    mir::Expr::return_cvoid()
                }
            }
            mir::Expr::Exprs(exprs) => mir::Expr::exprs(self.walk_exprs(exprs)?),
            mir::Expr::Cast(cast_type, expr) => mir::Expr::cast(cast_type, self.walk_expr(*expr)?),
            mir::Expr::CreateObject(_) => expr,
            mir::Expr::CreateTypeObject(_) => expr,
            mir::Expr::Unbox(e) => mir::Expr::unbox(self.walk_expr(*e)?),
            mir::Expr::RawI64(_) => expr,
            mir::Expr::Nop => expr,
            mir::Expr::StringLiteral(_) => expr,
            mir::Expr::CreateNativeArray(elem_exprs) => {
                let new_elem_exprs = self.walk_exprs(elem_exprs)?;
                (mir::Expr::CreateNativeArray(new_elem_exprs), expr.1.clone())
            }
            mir::Expr::NativeArrayRef(arr_expr, idx) => {
                let new_arr = self.walk_expr(*arr_expr)?;
                (
                    mir::Expr::NativeArrayRef(Box::new(new_arr), idx),
                    expr.1.clone(),
                )
            }
            mir::Expr::WTableKey(_) => expr,
            mir::Expr::WTableRow(_, _) => expr,
        };
        self.rewrite_expr(new_expr)
    }
}
