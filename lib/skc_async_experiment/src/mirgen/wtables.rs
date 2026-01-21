use crate::mir;
use crate::names::FunctionName;
use shiika_core::names::ClassFullname;
use skc_hir::SkTypes;

pub fn inserter_funcs(sk_types: &SkTypes) -> Vec<mir::Function> {
    let mut funcs = vec![];
    funcs.push(main_inserter(sk_types));
    for sk_class in sk_types.sk_classes() {
        if !sk_class.wtable.is_empty() {
            funcs.push(inserter_func(sk_class));
        }
    }
    funcs
}

fn inserter_func(sk_class: &skc_hir::SkClass) -> mir::Function {
    let mut body_stmts = vec![];
    for mod_name in sk_class.wtable.0.keys() {
        let len = sk_class.wtable.get_len(mod_name);
        body_stmts.push(mir::Expr::fun_call(
            mir::Expr::func_ref(
                FunctionName::mangled("shiika_insert_wtable"),
                mir::FunTy::sync(
                    vec![mir::Ty::Ptr, mir::Ty::Int64, mir::Ty::Ptr, mir::Ty::Int64],
                    mir::Ty::Ptr,
                ),
            ),
            vec![
                mir::Expr::const_ref(sk_class.fullname().to_const_fullname(), mir::Ty::Ptr),
                mir::Expr::wtable_key(mod_name.clone()),
                mir::Expr::wtable_row(sk_class.fullname(), mod_name.clone()),
                mir::Expr::raw_i64(len as i64),
            ],
        ));
    }
    body_stmts.push(mir::Expr::return_cvoid());
    let func_name = insert_wtable_func_name(&sk_class.fullname());
    mir::Function {
        asyncness: mir::Asyncness::Sync,
        name: FunctionName::mangled(func_name),
        params: vec![],
        ret_ty: mir::Ty::CVoid,
        body_stmts: mir::Expr::exprs(body_stmts),
        sig: None,
    }
}

/// Returns mir::Expr that calls shiika_insert_all_wtables()
pub fn call_main_inserter() -> mir::TypedExpr {
    mir::Expr::fun_call(
        mir::Expr::func_ref(
            main_inserter_name(),
            mir::FunTy::sync(vec![], mir::Ty::CVoid),
        ),
        vec![],
    )
}

fn main_inserter(sk_types: &SkTypes) -> mir::Function {
    let mut body_stmts = vec![];
    for sk_class in sk_types.sk_classes() {
        if !sk_class.wtable.is_empty() {
            body_stmts.push(mir::Expr::fun_call(
                mir::Expr::func_ref(
                    FunctionName::mangled(insert_wtable_func_name(&sk_class.fullname())),
                    mir::FunTy::sync(vec![], mir::Ty::CVoid),
                ),
                vec![],
            ));
        }
    }
    body_stmts.push(mir::Expr::return_cvoid());
    mir::Function {
        asyncness: mir::Asyncness::Sync,
        name: main_inserter_name(),
        params: vec![],
        ret_ty: mir::Ty::CVoid,
        body_stmts: mir::Expr::exprs(body_stmts),
        sig: None,
    }
}

fn main_inserter_name() -> FunctionName {
    FunctionName::mangled("shiika_insert_all_wtables")
}

fn insert_wtable_func_name(cls: &ClassFullname) -> String {
    format!("shiika_insert_{}_wtables", cls)
}
