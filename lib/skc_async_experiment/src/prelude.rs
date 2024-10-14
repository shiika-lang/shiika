use crate::hir::{self, FunTy, Ty};
use crate::names::FunctionName;
use anyhow::{Context, Result};
use shiika_parser;
use std::io::Read;

/// Functions that are called by the user code
pub fn lib_externs() -> Vec<(FunctionName, FunTy)> {
    let mut v = vec![
        // Built-in functions
        ("print", FunTy::sync(vec![Ty::Int], Ty::Void)),
        ("sleep_sec", FunTy::async_(vec![Ty::Int], Ty::Void)),
    ]
    .into_iter()
    .map(|(name, ty)| (FunctionName::Mangled(name.to_string()), ty))
    .collect::<Vec<_>>();
    v.append(&mut core_class_funcs());
    v
}

fn core_class_funcs() -> Vec<(FunctionName, FunTy)> {
    load_methods_json("lib/skc_runtime/")
        .unwrap()
        .into_iter()
        .map(|(class, method)| {
            let sig_str = format!("{}#{}", class, method);
            parse_sig(&sig_str)
        })
        .collect()
}

fn load_methods_json<P: AsRef<std::path::Path>>(dir: P) -> Result<Vec<(String, String)>> {
    let json_path = dir.as_ref().join("exports.json5");
    let mut f = std::fs::File::open(json_path).context("exports.json5 not found")?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("failed to read exports.json5")?;
    json5::from_str(&contents).context("exports.json5 is broken")
}

fn parse_sig(sig_str: &str) -> (FunctionName, FunTy) {
    let ast_sig = shiika_parser::Parser::parse_signature(sig_str).unwrap();
    let mut fun_ty = hir::untyped::signature_to_fun_ty(&ast_sig);
    // TODO: Support async rust libfunc
    fun_ty.asyncness = hir::Asyncness::Sync;
    (FunctionName::Mangled(ast_sig.name.to_string()), fun_ty)
}

/// Functions that are called by the generated code
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
            "chiika_env_ref",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64, Ty::Int64], Ty::Int),
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
