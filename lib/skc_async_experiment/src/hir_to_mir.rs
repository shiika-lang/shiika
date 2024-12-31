use crate::{hir, mir};

pub fn run(hir: hir::Program) -> mir::Program {
    let externs = hir
        .externs
        .into_iter()
        .map(|e| mir::Extern {
            name: e.name,
            fun_ty: convert_fun_ty(e.fun_ty),
        })
        .collect();
    let funcs = hir
        .funcs
        .into_iter()
        .map(|f| mir::Function {
            asyncness: convert_asyncness(f.asyncness),
            name: f.name,
            params: f.params.into_iter().map(|x| convert_param(x)).collect(),
            ret_ty: convert_ty(f.ret_ty),
            body_stmts: convert_texpr(f.body_stmts),
        })
        .collect();
    mir::Program::new(externs, funcs)
}

fn convert_fun_ty(fun_ty: hir::FunTy) -> mir::FunTy {
    mir::FunTy {
        asyncness: convert_asyncness(fun_ty.asyncness),
        param_tys: fun_ty
            .param_tys
            .into_iter()
            .map(|x| convert_ty(x))
            .collect(),
        ret_ty: Box::new(convert_ty(*fun_ty.ret_ty)),
    }
}

fn convert_asyncness(a: hir::Asyncness) -> mir::Asyncness {
    match a {
        hir::Asyncness::Unknown => mir::Asyncness::Unknown,
        hir::Asyncness::Sync => mir::Asyncness::Sync,
        hir::Asyncness::Async => mir::Asyncness::Async,
        hir::Asyncness::Lowered => mir::Asyncness::Lowered,
    }
}

fn convert_ty(ty: hir::Ty) -> mir::Ty {
    match ty {
        hir::Ty::Unknown => mir::Ty::Unknown,
        hir::Ty::Any => mir::Ty::Any,
        hir::Ty::Int64 => mir::Ty::Int64,
        hir::Ty::ChiikaEnv => mir::Ty::ChiikaEnv,
        hir::Ty::RustFuture => mir::Ty::RustFuture,
        hir::Ty::Raw(s) => mir::Ty::Raw(s),
        hir::Ty::Fun(fun_ty) => mir::Ty::Fun(convert_fun_ty(fun_ty)),
    }
}

fn convert_param(param: hir::Param) -> mir::Param {
    mir::Param {
        ty: convert_ty(param.ty),
        name: param.name,
    }
}

fn convert_texpr(texpr: hir::TypedExpr) -> mir::TypedExpr {
    (convert_expr(texpr.0), convert_ty(texpr.1))
}

fn convert_texpr_vec(exprs: Vec<hir::TypedExpr>) -> Vec<mir::TypedExpr> {
    exprs.into_iter().map(|x| convert_texpr(x)).collect()
}

fn convert_expr(expr: hir::Expr) -> mir::Expr {
    match expr {
        hir::Expr::Number(i) => mir::Expr::Number(i),
        hir::Expr::PseudoVar(p) => mir::Expr::PseudoVar(p),
        hir::Expr::LVarRef(s) => mir::Expr::LVarRef(s),
        hir::Expr::ArgRef(i, s) => mir::Expr::ArgRef(i, s),
        hir::Expr::EnvRef(i, s) => mir::Expr::EnvRef(i, s),
        hir::Expr::EnvSet(i, v, s) => mir::Expr::EnvSet(i, Box::new(convert_texpr(*v)), s),
        hir::Expr::FuncRef(n) => mir::Expr::FuncRef(n),
        hir::Expr::FunCall(f, a) => {
            mir::Expr::FunCall(Box::new(convert_texpr(*f)), convert_texpr_vec(a))
        }
        hir::Expr::If(c, t, e) => mir::Expr::If(
            Box::new(convert_texpr(*c)),
            Box::new(convert_texpr(*t)),
            Box::new(convert_texpr(*e)),
        ),
        hir::Expr::While(c, b) => {
            mir::Expr::While(Box::new(convert_texpr(*c)), Box::new(convert_texpr(*b)))
        }
        hir::Expr::Spawn(b) => mir::Expr::Spawn(Box::new(convert_texpr(*b))),
        hir::Expr::Alloc(s) => mir::Expr::Alloc(s),
        hir::Expr::Assign(s, v) => mir::Expr::Assign(s, Box::new(convert_texpr(*v))),
        hir::Expr::Return(v) => mir::Expr::Return(Box::new(convert_texpr(*v))),
        hir::Expr::Exprs(b) => mir::Expr::Exprs(convert_texpr_vec(b)),
        hir::Expr::Cast(t, v) => mir::Expr::Cast(t, Box::new(convert_texpr(*v))),
        _ => panic!("unexpected for hir_to_mir"),
    }
}
