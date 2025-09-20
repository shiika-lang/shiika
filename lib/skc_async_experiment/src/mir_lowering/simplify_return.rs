//! Convert `return foo()` to `tmp = foo(); return tmp;`
//!
//! This will simplifies handling of `return` in async_splitter especially
//! it is wrapped by `Cast()`.
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
        body_stmts: new_body_stmts,
        sig: orig_func.sig,
    }
}

struct Update {
    gensym_id: usize,
}
impl Update {
    fn new() -> Self {
        Update { gensym_id: 0 }
    }

    fn run(&mut self, expr: mir::TypedExpr) -> mir::TypedExpr {
        self.walk_expr(expr).unwrap()
    }

    fn gensym(&mut self) -> String {
        let id = self.gensym_id;
        self.gensym_id += 1;
        format!("$r{}", id)
    }
}
impl MirRewriter for Update {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let new_texpr = match texpr.0 {
            mir::Expr::Return(arg_expr) => match &arg_expr.0 {
                mir::Expr::Cast(_, _) | mir::Expr::FunCall(_, _) => {
                    let ret_ty = arg_expr.1.clone();
                    let tmp_name = self.gensym();
                    mir::Expr::exprs(vec![
                        mir::Expr::alloc(tmp_name.clone(), ret_ty.clone()),
                        mir::Expr::lvar_set(tmp_name.clone(), *arg_expr),
                        mir::Expr::return_(mir::Expr::lvar_ref(tmp_name, ret_ty)),
                    ])
                }
                _ => mir::Expr::return_(*arg_expr),
            },
            _ => texpr,
        };
        Ok(new_texpr)
    }
}
