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
use crate::names::FunctionName;
use anyhow::Result;
use std::collections::HashMap;

pub fn run(hir: hir::Program) -> hir::Program {
    let mut func_idx = HashMap::new();
    let mut externs = vec![];
    for e in hir.externs {
        func_idx.insert(e.name.clone(), e.is_async());
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

    let mut u = Update { func_idx };

    let funcs = hir
        .funcs
        .into_iter()
        .map(|f| compile_func(&mut u, f))
        .collect();
    hir::Program::new(externs, funcs)
}

/// Entry point for each milika function
fn compile_func(u: &mut Update, orig_func: hir::Function) -> hir::Function {
    let new_body_stmts = u.walk_expr(orig_func.body_stmts).unwrap();
    let new_params = insert_env_to_params(orig_func.params);
    hir::Function {
        generated: orig_func.generated,
        asyncness: orig_func.asyncness,
        name: orig_func.name,
        params: new_params,
        ret_ty: orig_func.ret_ty,
        body_stmts: new_body_stmts,
    }
}

struct Update {
    func_idx: HashMap<FunctionName, bool>,
}
impl HirRewriter for Update {
    fn rewrite_expr(&mut self, texpr: hir::TypedExpr) -> Result<hir::TypedExpr> {
        let mut new_texpr = match texpr.0 {
            hir::Expr::FunCall(fexpr, args) => {
                let mut new_args = args
                    .into_iter()
                    .map(|arg| self.walk_expr(arg))
                    .collect::<Result<Vec<_>>>()?;
                insert_env_to_args(&mut new_args);
                hir::Expr::fun_call(*fexpr, new_args)
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
    args.insert(0, hir::Expr::arg_ref(0, hir::Ty::ChiikaEnv));
}
