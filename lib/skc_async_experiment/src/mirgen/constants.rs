use crate::mir;
use crate::names::FunctionName;
use shiika_core::names::ConstFullname;

pub fn create_const_init_funcs(
    package: Option<&String>,
    constants: Vec<(ConstFullname, mir::TypedExpr)>,
) -> Vec<mir::Function> {
    let mut body_stmts = constants
        .iter()
        .map(|(name, _)| {
            mir::Expr::fun_call(
                mir::Expr::func_ref(
                    const_init_name(&name),
                    mir::FunTy::new(mir::Asyncness::Async, vec![], mir::Ty::Int64),
                ),
                vec![],
            )
        })
        .collect::<Vec<_>>();
    body_stmts.push(mir::Expr::return_(mir::Expr::raw_i64(0)));

    let mut funcs: Vec<_> = constants
        .into_iter()
        .map(|(name, rhs)| create_const_init_func(name, rhs))
        .collect();

    funcs.push(mir::Function {
        // PERF: Currently all const init functions are treated as async (safe side)
        asyncness: mir::Asyncness::Async,
        name: package_const_init_name(package),
        params: vec![],
        ret_ty: mir::Ty::Int64,
        body_stmts: mir::Expr::exprs(body_stmts),
        sig: None,
        lvar_count: None,
    });
    funcs
}

pub fn create_const_init_func(name: ConstFullname, rhs: mir::TypedExpr) -> mir::Function {
    let mut body_stmts = vec![];
    body_stmts.push(mir::Expr::const_set(name.clone(), rhs));
    body_stmts.push(mir::Expr::return_(mir::Expr::raw_i64(0)));
    mir::Function {
        // PERF: Currently all const init functions are treated as async (safe side)
        asyncness: mir::Asyncness::Async,
        name: const_init_name(&name),
        params: vec![],
        ret_ty: mir::Ty::Int64,
        body_stmts: mir::Expr::exprs(body_stmts),
        sig: None,
        lvar_count: None,
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
                mir::Ty::Int64,
            ),
        })
        .collect()
}

pub fn call_all_const_inits(total_deps: &[String]) -> Vec<mir::TypedExpr> {
    let mut names: Vec<_> = total_deps.into_iter().map(|name| Some(name)).collect();
    names.push(None); // None == const_inits for main
    names
        .into_iter()
        .map(|name| {
            let fname = package_const_init_name(name);
            let fun_ty = mir::FunTy::new(mir::Asyncness::Unknown, vec![], mir::Ty::Int64);
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

fn const_init_name(name: &ConstFullname) -> FunctionName {
    FunctionName::mangled(format!("shiika_init_const_{}", &name.0))
}
