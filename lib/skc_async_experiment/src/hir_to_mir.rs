use crate::names::FunctionName;
use crate::{hir, mir};
use shiika_core::ty::TermTy;
use skc_mir::LibraryExports;
use std::collections::HashSet;

pub fn run(hir: hir::CompilationUnit) -> mir::Program {
    let externs = convert_externs(hir.imports, hir.imported_asyncs);
    let funcs = hir
        .program
        .methods
        .into_iter()
        .map(|f| {
            let mut params = f
                .params
                .into_iter()
                .map(|x| convert_param(x))
                .collect::<Vec<_>>();
            if let Some(self_ty) = f.self_ty {
                params.insert(
                    0,
                    mir::Param {
                        ty: convert_ty(self_ty),
                        name: "self".to_string(),
                    },
                );
            }
            mir::Function {
                asyncness: mir::Asyncness::Unknown,
                name: f.name,
                params,
                ret_ty: convert_ty(f.ret_ty),
                body_stmts: convert_texpr(f.body_stmts),
            }
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
                let mut param_tys = sig
                    .params
                    .iter()
                    .map(|x| convert_ty(x.ty.clone()))
                    .collect::<Vec<_>>();
                param_tys.insert(0, convert_ty(sk_type.term_ty()));
                let fun_ty = mir::FunTy::new(asyncness, param_tys, convert_ty(sig.ret_ty.clone()));
                mir::Extern {
                    name: fname,
                    fun_ty,
                }
            })
        })
        .collect()
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
        hir::Expr::ArgRef(i, s) => {
            // +1 for the receiver
            mir::Expr::ArgRef(i + 1, s)
        }
        hir::Expr::ConstRef(resolved_const_name) => {
            // TODO: impl. constants
            mir::Expr::ConstRef(mir_const_name(resolved_const_name.names))
        }
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
        hir::Expr::ConstSet(name, rhs) => {
            mir::Expr::ConstSet(mir_const_name(name.names), Box::new(convert_texpr(*rhs)))
        }
        hir::Expr::Return(v) => mir::Expr::Return(Box::new(convert_texpr(*v))),
        hir::Expr::Exprs(b) => mir::Expr::Exprs(convert_texpr_vec(b)),
        hir::Expr::Upcast(v, t) => mir::Expr::Cast(
            mir::CastType::Upcast(convert_ty(t)),
            Box::new(convert_texpr(*v)),
        ),
        _ => panic!("unexpected for hir_to_mir"),
    }
}

fn mir_const_name(names: Vec<String>) -> String {
    "::".to_string() + &names.join("::")
}
