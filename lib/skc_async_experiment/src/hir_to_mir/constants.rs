use crate::mir;
use crate::names::FunctionName;
use shiika_core::names::ConstFullname;

pub fn create_const_init_func(
    package: Option<&String>,
    constants: Vec<(ConstFullname, mir::TypedExpr)>,
) -> mir::Function {
    let mut body_stmts: Vec<_> = constants
        .into_iter()
        .map(|(name, rhs)| mir::Expr::const_set(mir::mir_const_name(name), rhs))
        .collect();
    body_stmts.push(mir::Expr::return_(mir::Expr::number(0)));
    mir::Function {
        // PERF: Currently all const init functions are treated as async (safe side)
        asyncness: mir::Asyncness::Async,
        name: package_const_init_name(package),
        params: vec![],
        ret_ty: mir::Ty::Raw("Int".to_string()),
        body_stmts: mir::Expr::exprs(body_stmts),
        sig: None,
    }
}

pub fn const_init_externs(deps: &[String]) -> Vec<mir::Extern> {
    deps.iter()
        .map(|name| mir::Extern {
            name: package_const_init_name(Some(name)),
            fun_ty: mir::FunTy::new(
                // PERF: Currently all const init functions are treated as async (safe side)
                mir::Asyncness::Async,
                vec![],
                mir::Ty::Raw("Int".to_string()),
            ),
        })
        .collect()
}

pub fn call_all_const_inits(total_deps: &[String]) -> Vec<mir::TypedExpr> {
    total_deps
        .iter()
        .map(|name| {
            let fname = package_const_init_name(Some(name));
            let fun_ty = mir::FunTy::new(
                mir::Asyncness::Unknown,
                vec![],
                mir::Ty::Raw("Int".to_string()),
            );
            mir::Expr::fun_call(mir::Expr::func_ref(fname, fun_ty), vec![])
        })
        .collect()
}

fn package_const_init_name(package_name: Option<&String>) -> FunctionName {
    let suffix = if let Some(pkg) = package_name {
        format!("{}", pkg)
    } else {
        String::new()
    };
    FunctionName::mangled(format!("shiika_init_const_{}", suffix))
}
