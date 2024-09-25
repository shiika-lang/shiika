use crate::hir::{self, FunTy, Ty};

pub fn lib_externs() -> Vec<(&'static str, FunTy)> {
    vec![
        // Built-in functions
        ("print", FunTy::sync(vec![Ty::Int], Ty::Void)),
        ("sleep_sec", FunTy::async_(vec![Ty::Int], Ty::Void)),
    ]
}

pub fn core_externs() -> Vec<(&'static str, FunTy)> {
    let void_cont = FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Void], Ty::RustFuture);
    let spawnee = FunTy::lowered(
        vec![Ty::ChiikaEnv, void_cont.into(), Ty::RustFuture],
        Ty::RustFuture,
    );
    vec![
        ("GC_init", FunTy::lowered(vec![], Ty::Void)),
        ("shiika_malloc", FunTy::lowered(vec![Ty::Int64], Ty::Any)),
        (
            "chiika_env_push_frame",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64], Ty::Void),
        ),
        (
            "chiika_env_set",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64, Ty::Any, Ty::Int64], Ty::Void),
        ),
        (
            "chiika_env_pop_frame",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64], Ty::Any),
        ),
        (
            "chiika_env_get",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64, Ty::Int64], Ty::Any),
        ),
        (
            "chiika_spawn",
            FunTy::lowered(vec![spawnee.into(), Ty::RustFuture], Ty::Void),
        ),
        ("chiika_start_tokio", FunTy::lowered(vec![], Ty::Void)),
    ]
}

pub fn funcs(main_is_async: bool) -> Vec<hir::Function> {
    vec![
        hir::Function {
            name: "main".to_string(),
            generated: false,
            asyncness: hir::Asyncness::Lowered,
            params: vec![],
            ret_ty: Ty::Int64,
            body_stmts: main_body(),
        },
        hir::Function {
            name: "chiika_start_user".to_string(),
            generated: false,
            asyncness: hir::Asyncness::Lowered,
            params: vec![
                hir::Param::new(Ty::ChiikaEnv, "env"),
                hir::Param::new(
                    Ty::Fun(FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int], Ty::RustFuture)),
                    "cont",
                ),
            ],
            ret_ty: Ty::RustFuture,
            body_stmts: chiika_start_user_body(main_is_async),
        },
    ]
}

fn main_body() -> Vec<hir::TypedExpr> {
    let t = FunTy::lowered(vec![], Ty::Void);
    let chiika_start_tokio = hir::Expr::func_ref("chiika_start_tokio", t);
    vec![
        hir::Expr::fun_call(chiika_start_tokio, vec![]),
        // TODO: pass the resulting int to the user's main
        hir::Expr::return_(hir::Expr::unbox(hir::Expr::number(0))),
    ]
}

fn chiika_start_user_body(main_is_async: bool) -> Vec<hir::TypedExpr> {
    let cont_ty = FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int], Ty::RustFuture);
    let chiika_main = hir::Expr::func_ref(
        "chiika_main",
        if main_is_async {
            FunTy::lowered(
                vec![Ty::ChiikaEnv, Ty::Fun(cont_ty.clone())],
                Ty::RustFuture,
            )
        } else {
            FunTy::lowered(vec![], Ty::Int)
        },
    );
    let get_env = hir::Expr::arg_ref(0, Ty::ChiikaEnv);
    let get_cont = hir::Expr::arg_ref(1, Ty::Fun(cont_ty));
    let call = if main_is_async {
        hir::Expr::fun_call(chiika_main, vec![get_env, get_cont])
    } else {
        let call_sync_main = hir::Expr::fun_call(chiika_main, vec![]);
        hir::Expr::fun_call(get_cont, vec![get_env, call_sync_main])
    };
    vec![hir::Expr::return_(call)]
}
