//! Splice mir::Expr::Exprs into its body.
//!
//! ## Example
//!
//! ```
//! // Before
//! Exprs([f(), Exprs[g(), h()]]));
//! // After
//! Exprs([f(), g(), h()]);
use crate::mir;
use crate::mir::rewriter::MirRewriter;
use anyhow::Result;

pub fn run(mir: mir::Program) -> mir::Program {
    let funcs = mir.funcs.into_iter().map(compile_func).collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let new_body_stmts = Update::new().run(orig_func.body_stmts);
    mir::Function {
        asyncness: orig_func.asyncness,
        name: orig_func.name,
        params: orig_func.params,
        ret_ty: orig_func.ret_ty,
        body_stmts: splice_exprs(new_body_stmts),
        sig: orig_func.sig,
    }
}

struct Update();
impl Update {
    fn new() -> Self {
        Update()
    }

    fn run(&mut self, expr: mir::TypedExpr) -> mir::TypedExpr {
        self.walk_expr(expr).unwrap()
    }
}
impl MirRewriter for Update {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let new_texpr = match texpr.0 {
            mir::Expr::If(cond, then, else_) => {
                let new_then = splice_exprs(*then);
                let new_else = splice_exprs(*else_);
                mir::Expr::if_(*cond, new_then, new_else)
            }
            mir::Expr::While(cond, body) => {
                let new_body = splice_exprs(*body);
                mir::Expr::while_(*cond, new_body)
            }
            _ => texpr,
        };
        Ok(new_texpr)
    }
}

fn splice_exprs(exprs: mir::TypedExpr) -> mir::TypedExpr {
    mir::Expr::exprs(splice(exprs))
}

fn splice(exprs: mir::TypedExpr) -> Vec<mir::TypedExpr> {
    let mir::Expr::Exprs(expr_vec) = exprs.0 else {
        return vec![exprs];
    };
    let mut v = vec![];
    for e in expr_vec {
        match e.0 {
            mir::Expr::Exprs(inner_exprs) => {
                for ie in inner_exprs {
                    v.extend(splice(ie).into_iter());
                }
            }
            _ => v.push(e),
        }
    }
    v
}
