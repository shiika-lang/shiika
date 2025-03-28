mod collect_allocs;
use crate::names::FunctionName;
use crate::{hir, mir};
use shiika_core::names::ConstFullname;
use shiika_core::ty::TermTy;
use skc_mir::LibraryExports;
use std::collections::HashSet;

pub fn run(hir: hir::CompilationUnit) -> mir::CompilationUnit {
    log::debug!("Start");
    let vtables = skc_mir::VTables::build(&hir.sk_types, &hir.imports);
    log::debug!("VTables built");
    let externs = convert_externs(hir.imports, hir.imported_asyncs);
    let mut funcs: Vec<_> = hir
        .program
        .methods
        .into_iter()
        .map(convert_method)
        .collect();
    log::debug!("User functions converted");
    funcs.push(create_user_main(
        hir.program.top_exprs,
        hir.program.constants,
    ));
    let program = mir::Program::new(externs, funcs);
    mir::CompilationUnit { program, vtables }
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

fn convert_method(method: hir::Method<TermTy>) -> mir::Function {
    let mut params = method
        .params
        .into_iter()
        .map(|x| convert_param(x))
        .collect::<Vec<_>>();
    if let Some(self_ty) = method.self_ty {
        params.insert(
            0,
            mir::Param {
                ty: convert_ty(self_ty),
                name: "self".to_string(),
            },
        );
    }
    let allocs = collect_allocs::run(&method.body_stmts);
    let body_stmts = insert_allocs(allocs, convert_texpr(method.body_stmts));
    mir::Function {
        asyncness: mir::Asyncness::Unknown,
        name: method.name,
        params,
        ret_ty: convert_ty(method.ret_ty),
        body_stmts,
    }
}

fn insert_allocs(allocs: Vec<(String, TermTy)>, stmts: mir::TypedExpr) -> mir::TypedExpr {
    let mut stmts_vec = mir::expr::into_exprs(stmts);
    let mut new_stmts = vec![];
    for (name, ty) in allocs {
        new_stmts.push(mir::Expr::alloc(name, convert_ty(ty)));
    }
    new_stmts.extend(stmts_vec.drain(..));
    mir::Expr::exprs(new_stmts)
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
            mir::Expr::ConstRef(mir_const_name(resolved_const_name))
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
        hir::Expr::MethodCall(_, _, _) => todo!(),
        hir::Expr::While(c, b) => {
            mir::Expr::While(Box::new(convert_texpr(*c)), Box::new(convert_texpr(*b)))
        }
        hir::Expr::Spawn(b) => mir::Expr::Spawn(Box::new(convert_texpr(*b))),
        hir::Expr::LVarDecl(s, rhs) => mir::Expr::Assign(s, Box::new(convert_texpr(*rhs))),
        hir::Expr::Assign(s, v) => mir::Expr::Assign(s, Box::new(convert_texpr(*v))),
        hir::Expr::ConstSet(name, v) => {
            mir::Expr::ConstSet(mir_const_name(name), Box::new(convert_texpr(*v)))
        }
        hir::Expr::Return(v) => mir::Expr::Return(Box::new(convert_texpr(*v))),
        hir::Expr::Exprs(b) => mir::Expr::Exprs(convert_texpr_vec(b)),
        hir::Expr::Upcast(v, t) => mir::Expr::Cast(
            mir::CastType::Upcast(convert_ty(t)),
            Box::new(convert_texpr(*v)),
        ),
        hir::Expr::CreateObject(class_name) => mir::Expr::CreateObject(class_name.0),
        hir::Expr::CreateTypeObject(type_name) => mir::Expr::CreateTypeObject(type_name.0),
    }
}

fn create_user_main(
    top_exprs: Vec<hir::TypedExpr<TermTy>>,
    constants: Vec<(ConstFullname, hir::TypedExpr<TermTy>)>,
) -> mir::Function {
    let mut body_stmts = vec![];
    body_stmts.extend(
        constants
            .into_iter()
            .map(|(name, rhs)| mir::Expr::const_set(mir_const_name(name), convert_texpr(rhs))),
    );
    body_stmts.extend(top_exprs.into_iter().map(convert_texpr));
    body_stmts.push(mir::Expr::return_(mir::Expr::number(0)));
    mir::Function {
        asyncness: mir::Asyncness::Unknown,
        name: mir::main_function_name(),
        params: vec![],
        ret_ty: mir::Ty::Raw("Int".to_string()),
        body_stmts: mir::Expr::exprs(body_stmts),
    }
}

fn mir_const_name(name: ConstFullname) -> String {
    name.0
}
