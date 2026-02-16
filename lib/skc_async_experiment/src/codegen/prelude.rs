// TODO: move to codegen/

use crate::mir::{self, FunTy, Ty};
use crate::names::FunctionName;

// Index of ivar @name of Class
pub const IDX_CLASS_IVAR_NAME: usize = 0;

/// Functions defined as Shiika runtime (in packages/core/ext)
pub fn core_externs() -> Vec<mir::Extern> {
    let void_cont = FunTy::lowered(vec![Ty::ChiikaEnv, Ty::CVoid], Ty::RustFuture);
    let spawnee = FunTy::lowered(
        vec![Ty::ChiikaEnv, void_cont.into(), Ty::RustFuture],
        Ty::RustFuture,
    );
    vec![
        ("GC_init", FunTy::sync(vec![], Ty::CVoid)),
        ("shiika_malloc", FunTy::sync(vec![Ty::Int64], Ty::Ptr)),
        (
            "shiika_lookup_wtable",
            FunTy::sync(vec![Ty::Ptr, Ty::Int64, Ty::Int64], Ty::Ptr),
        ),
        (
            "shiika_insert_wtable",
            FunTy::sync(vec![Ty::Ptr, Ty::Int64, Ty::Ptr, Ty::Int64], Ty::Ptr),
        ),
        (
            "chiika_env_push_frame",
            FunTy::sync(vec![Ty::ChiikaEnv, Ty::Int64], Ty::CVoid),
        ),
        (
            "chiika_env_set",
            FunTy::sync(
                vec![Ty::ChiikaEnv, Ty::Int64, Ty::Any, Ty::Int64],
                Ty::CVoid,
            ),
        ),
        (
            "chiika_env_pop_frame",
            FunTy::sync(vec![Ty::ChiikaEnv, Ty::Int64], Ty::Any),
        ),
        (
            "chiika_env_ref",
            FunTy::sync(vec![Ty::ChiikaEnv, Ty::Int64, Ty::Int64], Ty::Any),
        ),
        (
            "chiika_spawn",
            FunTy::sync(vec![spawnee.into(), Ty::RustFuture], Ty::CVoid),
        ),
        ("chiika_start_tokio", FunTy::sync(vec![], Ty::CVoid)),
    ]
    .into_iter()
    .map(|(name, ty)| mir::Extern {
        name: FunctionName::mangled(name),
        fun_ty: ty,
    })
    .collect()
}

/// Functions defined in skc_async_experiment::codegen
pub fn intrinsic_externs() -> Vec<(FunctionName, FunTy)> {
    vec![
        (
            "shiika_intrinsic_box_int",
            FunTy::lowered(vec![Ty::Int64], Ty::raw("Int")),
        ),
        (
            "shiika_intrinsic_box_bool",
            FunTy::lowered(vec![Ty::I1], Ty::raw("Bool")),
        ),
    ]
    .into_iter()
    .map(|(name, ty)| (FunctionName::mangled(name), ty))
    .collect()
}

pub fn main_funcs() -> Vec<mir::Function> {
    vec![
        mir::Function {
            name: FunctionName::mangled("main"),
            asyncness: mir::Asyncness::Lowered,
            params: vec![],
            ret_ty: Ty::Int64,
            body_stmts: main_body(),
            sig: None,
            lvar_count: None,
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
            sig: None,
            lvar_count: None,
        },
    ]
}

fn main_body() -> mir::TypedExpr {
    let t = FunTy::lowered(vec![], Ty::CVoid);
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
