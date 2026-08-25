//! - Convert `LVarRef` to `EnvRef` and `Assign` to `EnvSet`.
//! - Insert `$env` to the beginning of the async function parameters.
//! - Insert `$env` to the beginning of the async funcall arguments.
//! - $env contains (in order):
//!   - $cont (continuation)
//!   - original arguments
//!   - local variables (lvars)
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
use crate::mir;
use crate::mir::rewriter::MirRewriter;
use anyhow::Result;

pub fn run(mir: mir::Program) -> mir::Program {
    let mut externs = vec![];
    for e in mir.externs {
        let new_e = if e.is_async() {
            mir::Extern {
                name: e.name,
                fun_ty: insert_env_to_fun_ty(&e.fun_ty),
            }
        } else {
            e
        };
        externs.push(new_e);
    }

    let funcs = mir
        .funcs
        .into_iter()
        .map(|f| if f.is_async() { compile_func(f) } else { f })
        .collect();
    mir::Program::new(mir.classes, externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let is_async = orig_func.is_async();
    // Count the number of lvars to store in the env
    let allocs = mir::visitor::LVarDecls::collect(&orig_func.body_stmts);
    let lvar_count = allocs.len();

    let new_body_stmts = Update {
        orig_arity: orig_func.params.len(),
        allocs,
    }
    .run(orig_func.body_stmts);
    let new_params = if is_async {
        insert_env_to_params(orig_func.params)
    } else {
        orig_func.params
    };
    mir::Function {
        asyncness: orig_func.asyncness,
        name: orig_func.name,
        params: new_params,
        ret_ty: orig_func.ret_ty,
        body_stmts: new_body_stmts,
        sig: orig_func.sig,
        lvar_count: Some(lvar_count),
    }
}

struct Update {
    orig_arity: usize,
    allocs: Vec<(String, mir::Ty)>,
}
impl Update {
    fn run(&mut self, expr: mir::TypedExpr) -> mir::TypedExpr {
        self.walk_expr(expr).unwrap()
    }

    /// Returns the position of the lvar in $env
    fn lvar_idx(&self, varname: &str) -> usize {
        let i = self
            .allocs
            .iter()
            .position(|(name, _)| name == varname)
            .unwrap_or_else(|| panic!("lvar '{}' not found in allocs: {:?}", varname, self.allocs));
        // +1 for $cont
        1 + self.orig_arity + i
    }
}
impl MirRewriter for Update {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let mut new_texpr = match texpr.0 {
            mir::Expr::LVarRef(ref varname) => {
                let i = self.lvar_idx(varname);
                mir::Expr::env_ref(i, varname, texpr.1.clone())
            }
            mir::Expr::ArgRef(idx, name) => {
                // +1 for $cont
                mir::Expr::env_ref(idx + 1, name, texpr.1)
            }
            mir::Expr::LVarDecl(varname, rhs, _) | mir::Expr::LVarSet(varname, rhs) => {
                let i = self.lvar_idx(&varname);
                mir::Expr::env_set(i, *rhs, varname)
            }
            mir::Expr::FunCall(fexpr, mut args) => {
                if fexpr.1.is_async_fun() == Some(true) {
                    insert_env_to_args(&mut args);
                }
                mir::Expr::fun_call(*fexpr, args)
            }
            _ => texpr,
        };
        if new_texpr.1.is_async_fun() == Some(true) {
            new_texpr.1 = insert_env_to_fun_ty(&new_texpr.1.as_fun_ty()).into();
        }
        Ok(new_texpr)
    }
}

fn insert_env_to_fun_ty(fun_ty: &mir::FunTy) -> mir::FunTy {
    debug_assert!(fun_ty.is_async());
    let mut param_tys = fun_ty.param_tys.clone();
    param_tys.insert(0, mir::Ty::ChiikaEnv);
    mir::FunTy {
        param_tys,
        ret_ty: fun_ty.ret_ty.clone(),
        asyncness: fun_ty.asyncness.clone(),
    }
}

fn insert_env_to_params(params: Vec<mir::Param>) -> Vec<mir::Param> {
    let mut new_params = params.clone();
    new_params.insert(0, mir::Param::new(mir::Ty::ChiikaEnv, "$env"));
    new_params
}

fn insert_env_to_args(args: &mut Vec<mir::TypedExpr>) {
    args.insert(0, mir::Expr::arg_ref(0, "$env", mir::Ty::ChiikaEnv));
}
