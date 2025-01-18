use crate::names::FunctionName;
use crate::{hir, mir};
use shiika_core::ty::TermTy;
use skc_mir::LibraryExports;
use std::collections::HashSet;

pub fn run(hir: hir::Program<TermTy>) -> mir::Program {
    let externs = convert_externs(hir.imports, hir.imported_asyncs);
    let funcs = hir
        .methods
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

fn convert_externs(
    imports: LibraryExports,
    imported_asyncs: Vec<FunctionName>,
) -> Vec<mir::Extern> {
    let asyncs: HashSet<FunctionName> = HashSet::from_iter(imported_asyncs);
    imports
        .sk_types
        .0
        .values()
        .flat_map(|sk_type| {
            sk_type.base().method_sigs.unordered_iter().map(|(sig, _)| {
                let fname = FunctionName::from_sig(sig);
                let asyncness = if asyncs.contains(&fname) {
                    mir::Asyncness::Async
                } else {
                    mir::Asyncness::Sync
                };
                let fun_ty = mir::FunTy::new(
                    asyncness,
                    sig.params
                        .iter()
                        .map(|x| convert_ty(x.ty.clone()))
                        .collect(),
                    convert_ty(sig.ret_ty.clone()),
                );
                mir::Extern {
                    name: fname,
                    fun_ty,
                }
            })
        })
        .collect()
}

fn convert_asyncness(a: hir::Asyncness) -> mir::Asyncness {
    match a {
        hir::Asyncness::Unknown => mir::Asyncness::Unknown,
        hir::Asyncness::Sync => mir::Asyncness::Sync,
        hir::Asyncness::Async => mir::Asyncness::Async,
        hir::Asyncness::Lowered => mir::Asyncness::Lowered,
    }
}

fn convert_ty(ty: TermTy) -> mir::Ty {
    match ty.fn_x_info() {
        Some(tys) => {
            let mut param_tys = tys
                .into_iter()
                .map(|x| convert_ty(x.clone()))
                .collect::<Vec<_>>();
            let ret_ty = param_tys.pop().unwrap();
            mir::Ty::Fun(mir::FunTy {
                asyncness: mir::Asyncness::Unknown,
                param_tys,
                ret_ty: Box::new(ret_ty),
            })
        }
        _ => mir::Ty::Raw(ty.fullname.0),
    }
}

fn convert_param(param: hir::Param) -> mir::Param {
    mir::Param {
        ty: convert_ty(param.ty),
        name: param.name,
    }
}

fn convert_texpr(texpr: hir::TypedExpr<TermTy>) -> mir::TypedExpr {
    (convert_expr(texpr.0), convert_ty(texpr.1))
}

fn convert_texpr_vec(exprs: Vec<hir::TypedExpr<TermTy>>) -> Vec<mir::TypedExpr> {
    exprs.into_iter().map(|x| convert_texpr(x)).collect()
}

fn convert_expr(expr: hir::Expr<TermTy>) -> mir::Expr {
    match expr {
        hir::Expr::Number(i) => mir::Expr::Number(i),
        hir::Expr::PseudoVar(p) => mir::Expr::PseudoVar(p),
        hir::Expr::LVarRef(s) => mir::Expr::LVarRef(s),
        hir::Expr::ArgRef(i, s) => mir::Expr::ArgRef(i, s),
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
        _ => panic!("unexpected for hir_to_mir"),
    }
}
