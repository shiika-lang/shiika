use crate::hir::{self, FunTy, Ty};
use crate::names::FunctionName;
use anyhow::{Context, Result};
use shiika_parser;
use std::io::Read;

/// Functions that are called by the user code
pub fn lib_externs(skc_runtime_dir: &std::path::Path) -> Result<Vec<(FunctionName, FunTy)>> {
    let mut v = vec![
        // Built-in functions
        ("print", FunTy::sync(vec![Ty::Int], Ty::Void)),
        ("sleep_sec", FunTy::async_(vec![Ty::Int], Ty::Void)),
    ]
    .into_iter()
    .map(|(name, ty)| (FunctionName::unmangled(name), ty))
    .collect::<Vec<_>>();
    v.append(&mut core_class_funcs(skc_runtime_dir)?);
    Ok(v)
}

fn core_class_funcs(skc_runtime_dir: &std::path::Path) -> Result<Vec<(FunctionName, FunTy)>> {
    load_methods_json(skc_runtime_dir)
        .unwrap()
        .into_iter()
        .map(|(class, method)| parse_sig(class, method))
        .collect::<Result<Vec<_>>>()
        .context(format!(
            "Failed to load skc_runtime/exports.json5 in {}",
            skc_runtime_dir.display()
        ))
}

fn load_methods_json(skc_runtime_dir: &std::path::Path) -> Result<Vec<(String, String)>> {
    let json_path = skc_runtime_dir.join("exports.json5");
    let mut f = std::fs::File::open(json_path).context("exports.json5 not found")?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("failed to read exports.json5")?;
    json5::from_str(&contents).context("exports.json5 is broken")
}

fn parse_sig(class: String, sig_str: String) -> Result<(FunctionName, FunTy)> {
    let ast_sig = shiika_parser::Parser::parse_signature(&sig_str)?;
    let mut fun_ty = hir::untyped::signature_to_fun_ty(&ast_sig);
    // TODO: Support async rust libfunc
    fun_ty.asyncness = hir::Asyncness::Sync;

    // TMP: Insert receiver
    fun_ty.param_tys.insert(0, Ty::Int);

    Ok((
        FunctionName::unmangled(format!("{}#{}", class, ast_sig.name.0)),
        fun_ty,
    ))
}

/// Functions that are called by the generated code
pub fn core_externs() -> Vec<(FunctionName, FunTy)> {
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
    .into_iter()
    .map(|(name, ty)| (FunctionName::mangled(name), ty))
    .collect()
}

pub fn funcs() -> Vec<hir::Function> {
    vec![
        hir::Function {
            name: FunctionName::mangled("main"),
            generated: false,
            asyncness: hir::Asyncness::Lowered,
            params: vec![],
            ret_ty: Ty::Int64,
            body_stmts: main_body(),
        },
        hir::Function {
            name: FunctionName::mangled("chiika_start_user"),
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
            body_stmts: chiika_start_user_body(),
        },
    ]
}

fn main_body() -> hir::TypedExpr {
    let t = FunTy::lowered(vec![], Ty::Void);
    let chiika_start_tokio = hir::Expr::func_ref(FunctionName::mangled("chiika_start_tokio"), t);
    hir::Expr::exprs(vec![
        hir::Expr::fun_call(chiika_start_tokio, vec![]),
        // TODO: pass the resulting int to the user's main
        hir::Expr::return_(hir::Expr::unbox(hir::Expr::number(0))),
    ])
}

fn chiika_start_user_body() -> hir::TypedExpr {
    let cont_ty = FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int], Ty::RustFuture);
    let chiika_main = hir::Expr::func_ref(
        FunctionName::unmangled("chiika_main"),
        FunTy::lowered(
            vec![Ty::ChiikaEnv, Ty::Fun(cont_ty.clone())],
            Ty::RustFuture,
        ),
    );
    let get_env = hir::Expr::arg_ref(0, "env", Ty::ChiikaEnv);
    let get_cont = hir::Expr::arg_ref(1, "cont", Ty::Fun(cont_ty));
    let call = hir::Expr::fun_call(chiika_main, vec![get_env, get_cont]);
    hir::Expr::return_(call)
}
