use crate::hir;
use crate::mir::{self, FunTy, Ty};
use crate::names::FunctionName;
use anyhow::{Context, Result};
use shiika_core::names::type_fullname;
use shiika_parser;
use skc_hir::MethodSignature;
use skc_mir::LibraryExports;
use std::io::Read;

/// Functions that are called by the user code
/// Returns hir::FunTy because type checker needs it
pub fn load_lib_externs(
    skc_runtime_dir: &std::path::Path,
    imports: &mut LibraryExports,
) -> Result<Vec<FunctionName>> {
    let mut imported_asyncs = vec![];
    for (type_name, sig_str, is_async) in load_methods_json(skc_runtime_dir)? {
        let sig = parse_sig(type_name.clone(), sig_str)?;
        if is_async {
            imported_asyncs.push(FunctionName::from_sig(&sig));
        }
        imports
            .sk_types
            .define_method(&type_fullname(type_name), sig);
    }
    Ok(imported_asyncs)
}

fn load_methods_json(skc_runtime_dir: &std::path::Path) -> Result<Vec<(String, String, bool)>> {
    let json_path = skc_runtime_dir.join("exports.json5");
    let mut f = std::fs::File::open(json_path.clone()).context("exports.json5 not found")?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("failed to read exports.json5")?;
    json5::from_str(&contents).context(format!("{} is broken", json_path.display()))
}

fn parse_sig(type_name: String, sig_str: String) -> Result<MethodSignature> {
    let ast_sig = shiika_parser::Parser::parse_signature(&sig_str)?;
    Ok(hir::untyped::compile_signature(type_name, &ast_sig))
}

/// Functions that are called by the generated code
pub fn core_externs() -> Vec<(FunctionName, FunTy)> {
    let void_cont = FunTy::lowered(vec![Ty::ChiikaEnv, Ty::raw("Void")], Ty::RustFuture);
    let spawnee = FunTy::lowered(
        vec![Ty::ChiikaEnv, void_cont.into(), Ty::RustFuture],
        Ty::RustFuture,
    );
    vec![
        ("GC_init", FunTy::lowered(vec![], Ty::raw("Void"))),
        ("shiika_malloc", FunTy::lowered(vec![Ty::Int64], Ty::Any)),
        (
            "chiika_env_push_frame",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64], Ty::raw("Void")),
        ),
        (
            "chiika_env_set",
            FunTy::lowered(
                vec![Ty::ChiikaEnv, Ty::Int64, Ty::Any, Ty::Int64],
                Ty::raw("Void"),
            ),
        ),
        (
            "chiika_env_pop_frame",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64], Ty::Any),
        ),
        (
            "chiika_env_ref",
            FunTy::lowered(vec![Ty::ChiikaEnv, Ty::Int64, Ty::Int64], Ty::raw("Int")),
        ),
        (
            "chiika_spawn",
            FunTy::lowered(vec![spawnee.into(), Ty::RustFuture], Ty::raw("Void")),
        ),
        (
            "chiika_start_tokio",
            FunTy::lowered(vec![], Ty::raw("Void")),
        ),
    ]
    .into_iter()
    .map(|(name, ty)| (FunctionName::mangled(name), ty))
    .collect()
}

pub fn funcs() -> Vec<mir::Function> {
    vec![
        mir::Function {
            name: FunctionName::mangled("main"),
            asyncness: mir::Asyncness::Lowered,
            params: vec![],
            ret_ty: Ty::Int64,
            body_stmts: main_body(),
        },
        mir::Function {
            name: FunctionName::mangled("chiika_start_user"),
            asyncness: mir::Asyncness::Lowered,
            params: vec![
                mir::Param::new(Ty::ChiikaEnv, "env"),
                mir::Param::new(
                    Ty::Fun(FunTy::lowered(
                        vec![Ty::ChiikaEnv, Ty::raw("Int")],
                        Ty::RustFuture,
                    )),
                    "cont",
                ),
            ],
            ret_ty: Ty::RustFuture,
            body_stmts: chiika_start_user_body(),
        },
    ]
}

fn main_body() -> mir::TypedExpr {
    let t = FunTy::lowered(vec![], Ty::raw("Void"));
    let chiika_start_tokio = mir::Expr::func_ref(FunctionName::mangled("chiika_start_tokio"), t);
    mir::Expr::exprs(vec![
        mir::Expr::fun_call(chiika_start_tokio, vec![]),
        // TODO: pass the resulting int to the user's main
        mir::Expr::return_(mir::Expr::unbox(mir::Expr::number(0))),
    ])
}

fn chiika_start_user_body() -> mir::TypedExpr {
    let cont_ty = FunTy::lowered(vec![Ty::ChiikaEnv, Ty::raw("Int")], Ty::RustFuture);
    let chiika_main = mir::Expr::func_ref(
        mir::main_function_name(),
        FunTy::lowered(
            vec![Ty::ChiikaEnv, Ty::Fun(cont_ty.clone())],
            Ty::RustFuture,
        ),
    );
    let get_env = mir::Expr::arg_ref(0, "env", Ty::ChiikaEnv);
    let get_cont = mir::Expr::arg_ref(1, "cont", Ty::Fun(cont_ty));
    let call = mir::Expr::fun_call(chiika_main, vec![get_env, get_cont]);
    mir::Expr::return_(call)
}
