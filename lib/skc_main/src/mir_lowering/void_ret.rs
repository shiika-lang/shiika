//! Lower sync `Void`-returning functions to an LLVM `ret void`
//! (`mir::Ty::CVoid`), so they no longer produce and return a `Void` value.
//!
//! `Void` is a singleton: any `Void` value is the `::Void` instance (a cheap
//! global load). A function that only ever returns that singleton can instead
//! return nothing. This pass:
//!
//! 1. For a function (and matching `extern`) that is **sync** and returns
//!    `Void`, changes its return type to `CVoid`. The decision is a pure
//!    function of `(ret == Void) && sync`, so every compilation unit — and
//!    every call site, local or imported — reaches the same conclusion and the
//!    cross-package ABI stays consistent (asyncness is part of the exported
//!    signature).
//! 2. At each call to such a function, the result becomes `CVoid` (no value).
//!    Since callers may use the result as a `Void` value, the call is wrapped
//!    as `Exprs([call, ::Void])` to restore the singleton. In a discard
//!    (statement) position the `::Void` load is unused and LLVM removes it.
//! 3. Inside a `CVoid` function, `return <v>` becomes evaluate-`v`-then-`ret
//!    void`.
//!
//! Only sync functions are touched. A method reachable through a vtable is
//! always async (see the "Async runtime" note in the hacking guide), so sync
//! methods are never virtual: this pass deals purely with direct calls and
//! never has to keep a vtable slot's signature consistent across overrides.
//! It must run after `asyncness_check` (which fills in each function's and
//! call site's asyncness) and before `async_splitter`.
use crate::mir;
use crate::mir::rewriter::MirRewriter;
use crate::names::FunctionName;
use anyhow::Result;

pub fn run(mut mir: mir::Program) -> mir::Program {
    // Imported sync Void methods are declared as externs; their defining
    // package compiles them to `ret void`, so declare them the same here.
    for e in &mut mir.externs {
        if is_sync_void(&e.fun_ty) && !constructs_void(&e.name) {
            e.fun_ty.ret_ty = Box::new(mir::Ty::CVoid);
        }
    }
    let funcs = mir
        .funcs
        .into_iter()
        .map(|f| {
            let ret_is_cvoid = f.asyncness == mir::Asyncness::Sync
                && f.ret_ty == mir::Ty::raw("Void")
                && !constructs_void(&f.name);
            let mut lower = Lower { ret_is_cvoid };
            let body_stmts = lower.walk_expr(f.body_stmts).unwrap();
            let ret_ty = if ret_is_cvoid {
                mir::Ty::CVoid
            } else {
                f.ret_ty
            };
            mir::Function {
                ret_ty,
                body_stmts,
                ..f
            }
        })
        .collect();
    mir::Program { funcs, ..mir }
}

/// True if `fun_ty` is a sync function returning `Void` (the form this pass
/// turns into a `CVoid` return).
fn is_sync_void(fun_ty: &mir::FunTy) -> bool {
    fun_ty.asyncness == mir::Asyncness::Sync && *fun_ty.ret_ty == mir::Ty::raw("Void")
}

/// `Meta:Void#new` returns the `::Void` singleton object itself (it is what
/// initializes the `::Void` constant that every Void value loads). Its `Void`
/// return is therefore a real value, not unit, and must stay a pointer — it is
/// the one Void-returning function this pass must leave alone.
fn constructs_void(name: &FunctionName) -> bool {
    name.to_string() == "Meta:Void#new"
}

struct Lower {
    /// Whether the function currently being rewritten now returns `CVoid`.
    ret_is_cvoid: bool,
}

impl MirRewriter for Lower {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let new_texpr = match texpr.0 {
            // A call to a sync Void function: make the callee return `CVoid`
            // and re-materialize the `::Void` singleton for the result.
            mir::Expr::FunCall(fexpr, args)
                if matches!(&fexpr.1, mir::Ty::Fun(ft) if is_sync_void(ft))
                    && !matches!(&fexpr.0, mir::Expr::FuncRef(n) if constructs_void(n)) =>
            {
                let (fexpr_node, fexpr_ty) = *fexpr;
                let mut fun_ty = fexpr_ty.into_fun_ty();
                fun_ty.ret_ty = Box::new(mir::Ty::CVoid);
                let call = mir::Expr::fun_call((fexpr_node, fun_ty.into()), args);
                mir::Expr::exprs(vec![call, mir::Expr::void_const_ref()])
            }
            // In a `CVoid` function, drop the returned value and `ret void`.
            mir::Expr::Return(Some(v)) if self.ret_is_cvoid => {
                mir::Expr::exprs(vec![*v, mir::Expr::return_cvoid()])
            }
            _ => texpr,
        };
        Ok(new_texpr)
    }
}
