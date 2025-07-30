//! Lowers EnvRef and EnvSet to FunCall.

use crate::mir;
use crate::mir::rewriter::MirRewriter;
use crate::names::FunctionName;
use anyhow::Result;

pub fn run(mir: mir::Program) -> mir::Program {
    let funcs = mir.funcs.into_iter().map(|f| compile_func(f)).collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let new_body_stmts = Update::run(orig_func.body_stmts);
    mir::Function {
        asyncness: orig_func.asyncness,
        name: orig_func.name,
        params: orig_func.params,
        ret_ty: orig_func.ret_ty,
        body_stmts: new_body_stmts,
        sig: orig_func.sig,
    }
}

struct Update();
impl Update {
    fn run(expr: mir::TypedExpr) -> mir::TypedExpr {
        Update().walk_expr(expr).unwrap()
    }
}
impl MirRewriter for Update {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let new_texpr = match texpr.0 {
            mir::Expr::EnvRef(idx, _) => call_chiika_env_ref(idx, texpr.1),
            mir::Expr::EnvSet(idx, expr, _) => call_chiika_env_set(idx, *expr),
            _ => texpr,
        };
        Ok(new_texpr)
    }
}

fn call_chiika_env_ref(idx: usize, val_ty: mir::Ty) -> mir::TypedExpr {
    let idx_native = mir::Expr::raw_i64(idx as i64);
    let type_id = mir::Expr::raw_i64(mir::Ty::raw("Int").type_id());
    let fun_ty = mir::FunTy {
        asyncness: mir::Asyncness::Lowered,
        param_tys: vec![mir::Ty::ChiikaEnv, mir::Ty::Int64, mir::Ty::Int64],
        ret_ty: Box::new(mir::Ty::Any),
    };
    let fname = FunctionName::mangled("chiika_env_ref");
    mir::Expr::cast(
        mir::CastType::AnyToVal(val_ty),
        mir::Expr::fun_call(
            mir::Expr::func_ref(fname, fun_ty),
            vec![arg_ref_env(), idx_native, type_id],
        ),
    )
}

fn call_chiika_env_set(idx: usize, val: mir::TypedExpr) -> mir::TypedExpr {
    let idx_native = mir::Expr::raw_i64(idx as i64);
    let type_id = mir::Expr::raw_i64(val.1.type_id());
    let cast_val = {
        let cast_type = match val.1 {
            mir::Ty::Raw(_) => mir::CastType::RawToAny,
            mir::Ty::Fun(_) => mir::CastType::FunToAny,
            _ => panic!("[BUG] don't know how to cast {:?} to Any", val),
        };
        mir::Expr::cast(cast_type, val)
    };
    let fun_ty = mir::FunTy {
        asyncness: mir::Asyncness::Lowered,
        param_tys: vec![
            mir::Ty::ChiikaEnv,
            mir::Ty::Int64,
            mir::Ty::Any,
            mir::Ty::Int64,
        ],
        ret_ty: Box::new(mir::Ty::raw("Void")),
    };
    let fname = FunctionName::mangled("chiika_env_set");
    mir::Expr::fun_call(
        mir::Expr::func_ref(fname, fun_ty),
        vec![arg_ref_env(), idx_native, cast_val, type_id],
    )
}

/// Get the `$env` that is 0-th param of async func
fn arg_ref_env() -> mir::TypedExpr {
    mir::Expr::arg_ref(0, "$env", mir::Ty::ChiikaEnv)
}
