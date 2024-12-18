//! - Convert `LVarRef` to `EnvRef` and `Assign` to `EnvSet`.
//! - Insert `$env` to the beginning of the async function parameters.
//! - Insert `$env` to the beginning of the async funcall arguments.
//!
//! Example
//! ```
//! // Before
//! fun foo(x) -> Int {
//!   sleep_sec(1);
//!   return 42;
//! }
//! fun bar() -> Int {
//!   foo(1);
//!   return 43;
//! }
//! // After
//! fun foo($env, x) -> RustFuture {
//!   sleep_sec(1);
//!   return 42;
//! }
//! fun bar($env) -> RustFuture {
//!   foo($env, 1);
//!   return 43;
//! }
//! ```
use crate::hir;
use crate::hir::rewriter::HirRewriter;
use anyhow::Result;

pub fn run(hir: hir::Program) -> hir::Program {
    let mut externs = vec![];
    for e in hir.externs {
        let new_e = if e.is_async() {
            hir::Extern {
                name: e.name,
                fun_ty: insert_env_to_fun_ty(&e.fun_ty),
            }
        } else {
            e
        };
        externs.push(new_e);
    }

    let funcs = hir
        .funcs
        .into_iter()
        .map(|f| {
            if f.asyncness.is_async() {
                compile_func(f)
            } else {
                f
            }
        })
        .collect();
    hir::Program::new(externs, funcs)
}

fn compile_func(orig_func: hir::Function) -> hir::Function {
    let allocs = hir::visitor::Allocs::collect(&orig_func.body_stmts);
    let new_body_stmts = Update {
        orig_arity: orig_func.params.len(),
        allocs,
    }
    .run(orig_func.body_stmts);
    let new_params = if orig_func.asyncness.is_async() {
        insert_env_to_params(orig_func.params)
    } else {
        orig_func.params
    };
    hir::Function {
        asyncness: orig_func.asyncness,
        name: orig_func.name,
        params: new_params,
        ret_ty: orig_func.ret_ty,
        body_stmts: new_body_stmts,
    }
}

struct Update {
    orig_arity: usize,
    allocs: Vec<(String, hir::Ty)>,
}
impl Update {
    fn run(&mut self, expr: hir::TypedExpr) -> hir::TypedExpr {
        self.walk_expr(expr).unwrap()
    }

    /// Returns the position of the lvar in $env
    fn lvar_idx(&self, varname: &str) -> usize {
        let i = self
            .allocs
            .iter()
            .position(|(name, _)| name == varname)
            .expect("[BUG] lvar not in self.lvars");
        // +1 for $cont
        1 + self.orig_arity + i
    }
}
impl HirRewriter for Update {
    fn rewrite_expr(&mut self, texpr: hir::TypedExpr) -> Result<hir::TypedExpr> {
        let mut new_texpr = match texpr.0 {
            hir::Expr::LVarRef(ref varname) => {
                let i = self.lvar_idx(varname);
                hir::Expr::env_ref(i, varname, texpr.1.clone())
            }
            hir::Expr::ArgRef(idx, name) => hir::Expr::env_ref(idx + 1, name, texpr.1),
            hir::Expr::Assign(varname, rhs) => {
                let i = self.lvar_idx(&varname);
                hir::Expr::env_set(i, *rhs, varname)
            }
            hir::Expr::FunCall(fexpr, mut args) => {
                if fexpr.1.is_async_fun() == Some(true) {
                    insert_env_to_args(&mut args);
                }
                hir::Expr::fun_call(*fexpr, args)
            }
            _ => texpr,
        };
        if new_texpr.1.is_async_fun() == Some(true) {
            new_texpr.1 = insert_env_to_fun_ty(&new_texpr.1.as_fun_ty()).into();
        }
        Ok(new_texpr)
    }
}

fn insert_env_to_fun_ty(fun_ty: &hir::FunTy) -> hir::FunTy {
    debug_assert!(fun_ty.asyncness.is_async());
    let mut param_tys = fun_ty.param_tys.clone();
    param_tys.insert(0, hir::Ty::ChiikaEnv);
    hir::FunTy {
        param_tys,
        ret_ty: fun_ty.ret_ty.clone(),
        asyncness: fun_ty.asyncness.clone(),
    }
}

fn insert_env_to_params(params: Vec<hir::Param>) -> Vec<hir::Param> {
    let mut new_params = params.clone();
    new_params.insert(0, hir::Param::new(hir::Ty::ChiikaEnv, "$env"));
    new_params
}

fn insert_env_to_args(args: &mut Vec<hir::TypedExpr>) {
    args.insert(0, hir::Expr::arg_ref(0, "$env", hir::Ty::ChiikaEnv));
}
