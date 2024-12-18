//! Lowers EnvRef and EnvSet to FunCall.

use crate::hir;
use crate::hir::rewriter::HirRewriter;
use crate::names::FunctionName;
use anyhow::Result;

pub fn run(hir: hir::Program) -> hir::Program {
    let funcs = hir.funcs.into_iter().map(|f| compile_func(f)).collect();
    hir::Program::new(hir.externs, funcs)
}

fn compile_func(orig_func: hir::Function) -> hir::Function {
    let new_body_stmts = Update::run(orig_func.body_stmts);
    hir::Function {
        asyncness: orig_func.asyncness,
        name: orig_func.name,
        params: orig_func.params,
        ret_ty: orig_func.ret_ty,
        body_stmts: new_body_stmts,
    }
}

struct Update();
impl Update {
    fn run(expr: hir::TypedExpr) -> hir::TypedExpr {
        Update().walk_expr(expr).unwrap()
    }
}
impl HirRewriter for Update {
    fn rewrite_expr(&mut self, texpr: hir::TypedExpr) -> Result<hir::TypedExpr> {
        let new_texpr = match texpr.0 {
            hir::Expr::EnvRef(idx, _) => call_chiika_env_ref(idx),
            hir::Expr::EnvSet(idx, expr, _) => call_chiika_env_set(idx, *expr),
            _ => texpr,
        };
        Ok(new_texpr)
    }
}

fn call_chiika_env_ref(idx: usize) -> hir::TypedExpr {
    let idx_native = hir::Expr::raw_i64(idx as i64);
    let type_id = hir::Expr::raw_i64(hir::Ty::Int.type_id());
    let fun_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Int64, hir::Ty::Int64],
        // Milika lvars are all int
        ret_ty: Box::new(hir::Ty::Int),
    };
    let fname = FunctionName::mangled("chiika_env_ref");
    hir::Expr::fun_call(
        hir::Expr::func_ref(fname, fun_ty),
        vec![arg_ref_env(), idx_native, type_id],
    )
}

fn call_chiika_env_set(idx: usize, val: hir::TypedExpr) -> hir::TypedExpr {
    let idx_native = hir::Expr::raw_i64(idx as i64);
    let type_id = hir::Expr::raw_i64(val.1.type_id());
    let cast_val = {
        let cast_type = match val.1 {
            hir::Ty::Void => hir::CastType::VoidToAny,
            hir::Ty::Int => hir::CastType::IntToAny,
            hir::Ty::Fun(_) => hir::CastType::FunToAny,
            _ => panic!("[BUG] don't know how to cast {:?} to Any", val),
        };
        hir::Expr::cast(cast_type, val)
    };
    let fun_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![
            hir::Ty::ChiikaEnv,
            hir::Ty::Int64,
            hir::Ty::Any,
            hir::Ty::Int64,
        ],
        ret_ty: Box::new(hir::Ty::Void),
    };
    let fname = FunctionName::mangled("chiika_env_set");
    hir::Expr::fun_call(
        hir::Expr::func_ref(fname, fun_ty),
        vec![arg_ref_env(), idx_native, cast_val, type_id],
    )
}

/// Get the `$env` that is 0-th param of async func
fn arg_ref_env() -> hir::TypedExpr {
    hir::Expr::arg_ref(0, "$env", hir::Ty::ChiikaEnv)
}
