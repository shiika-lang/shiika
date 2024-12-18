//! Example
//! ```
//! // Before
//! fun foo($env) -> Int {
//!   print(sleep_sec(1));
//!   return 42;
//! }
//! // After
//! fun foo($env, $cont) -> RustFuture {
//!   chiika_env_push_frame($env, 1);
//!   chiika_env_set($env, 0, $cont);
//!   return sleep_sec(foo_1, 1);
//! }
//! fun foo_1($env, $async_result) -> RustFuture {
//!   print($async_result);
//!   return chiika_env_pop_frame($env)(42); // Call the original $cont
//! }
//! ```
use crate::hir;
use crate::names::FunctionName;
use anyhow::{anyhow, Result};
use std::collections::VecDeque;

/// Splits asynchronous Milika func into multiple funcs.
/// Also, signatures of async externs are modified to take `$env` and `$cont` as the first two params.
pub fn run(hir: hir::Program) -> Result<hir::Program> {
    let externs = hir
        .externs
        .into_iter()
        .map(|e| {
            if e.is_async() {
                hir::Extern {
                    fun_ty: hir::FunTy::lowered(
                        append_async_params(&e.fun_ty),
                        hir::Ty::RustFuture,
                    ),
                    ..e
                }
            } else {
                e
            }
        })
        .collect();

    let mut funcs = vec![];
    for mut f in hir.funcs {
        let allocs = hir::visitor::Allocs::collect(&f.body_stmts);
        // Extract body_stmts from f
        let mut body_stmts = hir::Expr::nop();
        std::mem::swap(&mut f.body_stmts, &mut body_stmts);
        let mut c = Compiler {
            orig_func: &mut f,
            allocs,
            chapters: Chapters::new(),
            gensym_ct: 0,
        };
        let mut split_funcs = c.compile_func(body_stmts)?;
        funcs.append(&mut split_funcs);
    }
    Ok(hir::Program::new(externs, funcs))
}

#[derive(Debug)]
struct Compiler<'a> {
    orig_func: &'a mut hir::Function,
    allocs: Vec<(String, hir::Ty)>,
    chapters: Chapters,
    gensym_ct: usize,
}

impl<'a> Compiler<'a> {
    /// Entry point for each milika function
    fn compile_func(&mut self, body_stmts: hir::TypedExpr) -> Result<Vec<hir::Function>> {
        self.chapters.add(Chapter::new_original(self.orig_func));
        if self.orig_func.asyncness.is_async() {
            self._compile_async_intro();
        }
        self.compile_stmts(hir::expr::into_exprs(body_stmts))?;
        let chaps = self.chapters.extract();
        Ok(chaps
            .into_iter()
            .map(|c| self._serialize_chapter(c))
            .collect())
    }

    fn _compile_async_intro(&mut self) {
        let arity = self.orig_func.params.len();
        self.chapters
            .add_stmt(call_chiika_env_push_frame(self.frame_size()));
        self.chapters.add_stmt(hir::Expr::env_set(
            0,
            arg_ref_cont(arity, self.orig_func.ret_ty.clone()),
            "$cont",
        ));
        for i in 1..arity {
            let param = &self.orig_func.params[i];
            self.chapters.add_stmt(hir::Expr::env_set(
                i,
                hir::Expr::arg_ref(i, &param.name, param.ty.clone()),
                &param.name,
            ));
        }
    }

    fn _serialize_chapter(&self, chap: Chapter) -> hir::Function {
        hir::Function {
            asyncness: hir::Asyncness::Lowered,
            name: FunctionName::unmangled(chap.name.clone()),
            params: chap.params,
            ret_ty: chap.ret_ty,
            body_stmts: hir::Expr::exprs(chap.stmts),
        }
    }

    fn compile_value_expr(&mut self, e: hir::TypedExpr, on_return: bool) -> Result<hir::TypedExpr> {
        if let Some(expr) = self.compile_expr(e, on_return)? {
            Ok(expr)
        } else {
            Err(anyhow!("Got None in compile_value_expr (async call?)"))
        }
    }

    /// Examine each expression for special care. Most important part is
    /// calling async function.
    fn compile_expr(
        &mut self,
        e: hir::TypedExpr,
        on_return: bool,
    ) -> Result<Option<hir::TypedExpr>> {
        let new_e = match e.0 {
            hir::Expr::Number(_) => e,
            hir::Expr::PseudoVar(_) => e,
            hir::Expr::ArgRef(_, _) => e,
            hir::Expr::EnvRef(_, _) => e,
            hir::Expr::EnvSet(idx, rhs, name) => {
                let v = self.compile_value_expr(*rhs, false)?;
                hir::Expr::env_set(idx, v, name)
            }
            hir::Expr::FuncRef(_) => e,
            hir::Expr::FunCall(fexpr, arg_exprs) => {
                let new_fexpr = self.compile_value_expr(*fexpr, false)?;
                let new_args = arg_exprs
                    .into_iter()
                    .map(|x| self.compile_value_expr(x, false))
                    .collect::<Result<Vec<_>>>()?;
                let fun_ty = new_fexpr.1.as_fun_ty();
                // No need to create a new chapter if on_return is true.
                // In that case the args are modified later (see hir::Expr::Return)
                if fun_ty.asyncness.is_async() && !on_return {
                    self.compile_async_call(new_fexpr, new_args, e.1)?
                } else {
                    hir::Expr::fun_call(new_fexpr, new_args)
                }
            }
            hir::Expr::If(cond_expr, then_exprs, else_exprs) => {
                return self.compile_if(&e.1, *cond_expr, *then_exprs, *else_exprs);
            }
            hir::Expr::While(_cond_expr, _body_exprs) => todo!(),
            hir::Expr::Spawn(fexpr) => {
                let new_fexpr = self.compile_value_expr(*fexpr, false)?;
                call_chiika_spawn(new_fexpr)
            }
            hir::Expr::Alloc(_) => hir::Expr::nop(),
            hir::Expr::Return(expr) => return self.compile_return(*expr),
            hir::Expr::Exprs(_) => {
                panic!("Exprs must be handled by its parent");
            }
            _ => panic!("unexpected for async_splitter: {:?}", e.0),
        };
        Ok(Some(new_e))
    }

    /// On calling an async function, create a new chapter and
    /// append the async call to the current chapter
    fn compile_async_call(
        &mut self,
        fexpr: hir::TypedExpr,
        args: Vec<hir::TypedExpr>,
        async_result_ty: hir::Ty,
    ) -> Result<hir::TypedExpr> {
        // Change chapter here
        let next_chapter_name = chapter_func_name(&self.orig_func.name, self.chapters.len());
        let last_chapter = self.chapters.last_mut();
        let terminator = hir::Expr::return_(modify_async_call(fexpr, args, next_chapter_name));
        last_chapter.stmts.push(terminator);
        last_chapter.async_result_ty = Some(async_result_ty.clone());
        self.chapters.add(Chapter::new_async_call_receiver(
            chapter_func_name(&self.orig_func.name, self.chapters.len()),
            async_result_ty.clone(),
        ));

        Ok(arg_ref_async_result(async_result_ty))
    }

    /// Compile a list of statements. It may contain an async call.
    fn compile_stmts(&mut self, stmts: Vec<hir::TypedExpr>) -> Result<()> {
        for stmt in stmts {
            if let Some(new_stmt) = self.compile_expr(stmt, false)? {
                self.chapters.add_stmt(new_stmt);
            }
        }
        Ok(())
    }

    /// Compile a list of expressions which does not contain async calls
    /// into Exprs.
    fn compile_exprs(&mut self, exprs_: hir::TypedExpr) -> Result<hir::TypedExpr> {
        let exprs = hir::expr::into_exprs(exprs_);
        let mut new_exprs = vec![];
        for expr in exprs {
            let Some(new_expr) = self.compile_expr(expr, false)? else {
                panic!("got None in compile_exprs (async call?)");
            };
            new_exprs.push(new_expr);
        }
        Ok(hir::Expr::exprs(new_exprs))
    }

    fn compile_if(
        &mut self,
        if_ty: &hir::Ty,
        cond_expr: hir::TypedExpr,
        then_exprs: hir::TypedExpr,
        else_exprs_: hir::TypedExpr,
    ) -> Result<Option<hir::TypedExpr>> {
        let new_cond_expr = self.compile_value_expr(cond_expr, false)?;
        if self.orig_func.asyncness.is_sync() {
            let then = self.compile_exprs(then_exprs)?;
            let els = self.compile_exprs(else_exprs_)?;
            return Ok(Some(hir::Expr::if_(new_cond_expr, then, els)));
        }

        let func_name = self.chapters.current_name().to_string();

        let then_chap = Chapter::new_async_if_clause(func_name.clone(), "t");
        let else_chap = Chapter::new_async_if_clause(func_name.clone(), "f");
        // Statements after `if` goes to an "endif" chapter
        let endif_chap = Chapter::new_async_end_if(func_name.clone(), "e", if_ty.clone()); // e for endif

        let fcall_t = self.branch_call(&then_chap.name);
        let fcall_f = self.branch_call(&else_chap.name);
        let terminator = hir::Expr::if_(
            new_cond_expr,
            hir::Expr::return_(fcall_t),
            hir::Expr::return_(fcall_f),
        );
        self.chapters.add_stmt(terminator);

        self.chapters.add(then_chap);
        self.compile_if_clause(then_exprs, &endif_chap.name)?;
        self.chapters.add(else_chap);
        self.compile_if_clause(else_exprs_, &endif_chap.name)?;

        if *if_ty == hir::Ty::Never {
            // Both branches end with return
            Ok(None)
        } else {
            self.chapters.add(endif_chap);
            Ok(Some(hir::Expr::arg_ref(1, "$ifResult", if_ty.clone())))
        }
    }

    fn compile_if_clause(&mut self, exprs_: hir::TypedExpr, endif_chap_name: &str) -> Result<()> {
        let mut exprs = hir::expr::into_exprs(exprs_);
        let e = exprs.pop().unwrap();
        let opt_vexpr = if e.1 == hir::Ty::Never {
            exprs.push(e);
            None
        } else {
            Some(e)
        };
        self.compile_stmts(exprs)?;
        // Send the value to the endif chapter (unless ends with `return`)
        if let Some(vexpr) = opt_vexpr {
            let new_vexpr = self.compile_value_expr(vexpr, false)?;
            let goto_endif = hir::Expr::fun_call(
                hir::Expr::func_ref(
                    FunctionName::unmangled(endif_chap_name),
                    hir::FunTy {
                        asyncness: hir::Asyncness::Lowered,
                        param_tys: vec![hir::Ty::ChiikaEnv, new_vexpr.1.clone()],
                        ret_ty: Box::new(hir::Ty::RustFuture),
                    },
                ),
                vec![arg_ref_env(), new_vexpr],
            );
            self.chapters.add_stmt(hir::Expr::return_(goto_endif));
        }
        Ok(())
    }

    /// Generate a call to the if-branch function
    fn branch_call(&self, chap_name: &str) -> hir::TypedExpr {
        let args = vec![arg_ref_env()];
        let chap_fun_ty = hir::FunTy {
            asyncness: self.orig_func.asyncness.clone(),
            param_tys: vec![hir::Ty::ChiikaEnv],
            ret_ty: Box::new(hir::Ty::RustFuture),
        };
        let fname = FunctionName::unmangled(chap_name.to_string());
        hir::Expr::fun_call(hir::Expr::func_ref(fname, chap_fun_ty), args)
    }

    fn compile_return(&mut self, expr: hir::TypedExpr) -> Result<Option<hir::TypedExpr>> {
        // `return return 1` == `return 1`
        if expr.1 == hir::Ty::Never {
            return self.compile_expr(expr, false);
        }
        let new_expr = self.compile_value_expr(expr, true)?;
        if self.orig_func.asyncness.is_sync() {
            return Ok(Some(hir::Expr::return_(new_expr)));
        }
        let env_pop = {
            let cont_ty = hir::Ty::Fun(hir::FunTy {
                asyncness: hir::Asyncness::Lowered,
                param_tys: vec![hir::Ty::ChiikaEnv, self.orig_func.ret_ty.clone()],
                ret_ty: Box::new(hir::Ty::RustFuture),
            });
            call_chiika_env_pop_frame(self.frame_size(), cont_ty)
        };
        let value_expr = if new_expr.0.is_async_fun_call() {
            // Convert `callee($env, args...)`
            // to `callee($env, args..., env_pop())`
            let hir::Expr::FunCall(fexpr, mut args) = new_expr.0 else {
                unreachable!();
            };
            args.push(env_pop);
            let new_fexpr = (fexpr.0, async_fun_ty(fexpr.1.as_fun_ty()).into());
            hir::Expr::fun_call(new_fexpr, args)
        } else {
            // alloc tmp;    // tmp is needed because
            // tmp = value   // calculating value may call env_ref
            // `(env_pop())(env, tmp)`
            let tmp = self.store_to_tmpvar(new_expr);
            hir::Expr::fun_call(env_pop, vec![arg_ref_env(), tmp])
        };
        Ok(Some(hir::Expr::return_(value_expr)))
    }

    /// Store the value to a temporary variable and return the varref
    fn store_to_tmpvar(&mut self, value: hir::TypedExpr) -> hir::TypedExpr {
        let ty = value.1.clone();
        let varname = self.gensym();
        self.chapters.add_stmts(vec![
            hir::Expr::alloc(varname.clone()),
            hir::Expr::assign(varname.clone(), value),
        ]);
        hir::Expr::lvar_ref(varname, ty)
    }

    fn gensym(&mut self) -> String {
        let n = self.gensym_ct;
        self.gensym_ct += 1;
        format!("${n}")
    }

    fn frame_size(&self) -> usize {
        // +1 for $cont
        1 + self.orig_func.params.len() + self.allocs.len()
    }
}

// Convert `Fun(ChiikaEnv, (X)->Y)` to `Fun((ChiikaEnv, X, Fun((ChiikaEnv, Y)->RustFuture))->RustFuture)`
fn async_fun_ty(orig_fun_ty: &hir::FunTy) -> hir::FunTy {
    let mut param_tys = orig_fun_ty.param_tys.clone();
    param_tys.push(hir::Ty::Fun(hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, *orig_fun_ty.ret_ty.clone()],
        ret_ty: Box::new(hir::Ty::RustFuture),
    }));
    hir::FunTy {
        asyncness: hir::Asyncness::Async,
        param_tys,
        ret_ty: Box::new(hir::Ty::RustFuture),
    }
}

fn modify_async_call(
    fexpr: hir::TypedExpr,
    mut args: Vec<hir::TypedExpr>,
    next_chapter_name: FunctionName,
) -> hir::TypedExpr {
    let hir::Ty::Fun(fun_ty) = &fexpr.1 else {
        panic!("[BUG] not a function: {:?}", fexpr.0);
    };
    // Append `$cont` (i.e. the next chapter)
    let next_chapter = {
        let next_chapter_ty = hir::FunTy {
            asyncness: hir::Asyncness::Lowered,
            param_tys: vec![hir::Ty::ChiikaEnv, *fun_ty.ret_ty.clone()],
            ret_ty: Box::new(hir::Ty::RustFuture),
        };
        hir::Expr::func_ref(next_chapter_name, next_chapter_ty)
    };
    args.push(next_chapter);
    let new_fexpr = (fexpr.0, async_fun_ty(fexpr.1.as_fun_ty()).into());
    hir::Expr::fun_call(new_fexpr, args)
}

/// Append param for async libfunc (i.e. `$cont`)
fn append_async_params(fun_ty: &hir::FunTy) -> Vec<hir::Ty> {
    let mut new_params = fun_ty.param_tys.to_vec();
    let cont_ty = hir::FunTy::lowered(
        vec![hir::Ty::ChiikaEnv, *fun_ty.ret_ty.clone()],
        hir::Ty::RustFuture,
    );
    new_params.push(hir::Ty::Fun(cont_ty));
    new_params
}

/// Create name of generated function like `foo_1`
fn chapter_func_name(orig_name: &FunctionName, chapter_idx: usize) -> FunctionName {
    FunctionName::unmangled(format!("{}_{}", orig_name, chapter_idx))
}

/// Get the `$env` that is 0-th param of async func
fn arg_ref_env() -> hir::TypedExpr {
    hir::Expr::arg_ref(0, "$env", hir::Ty::ChiikaEnv)
}

/// Get the `$cont` param of async func
/// The continuation takes an argument.
fn arg_ref_cont(arity: usize, arg_ty: hir::Ty) -> hir::TypedExpr {
    let cont_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, arg_ty],
        ret_ty: Box::new(hir::Ty::RustFuture),
    };
    hir::Expr::arg_ref(arity, "$cont", hir::Ty::Fun(cont_ty))
}

/// Get the `$async_result` which is 1-th param of chapter func
fn arg_ref_async_result(ty: hir::Ty) -> hir::TypedExpr {
    hir::Expr::arg_ref(1, "$async_result", ty)
}

fn call_chiika_env_push_frame(size: usize) -> hir::TypedExpr {
    let size_native = hir::Expr::raw_i64(size as i64);
    hir::Expr::fun_call(
        hir::Expr::func_ref(
            FunctionName::mangled("chiika_env_push_frame"),
            hir::FunTy {
                asyncness: hir::Asyncness::Lowered,
                param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Int64],
                ret_ty: Box::new(hir::Ty::Void),
            },
        ),
        vec![arg_ref_env(), size_native],
    )
}

fn call_chiika_env_pop_frame(n_pop: usize, popped_value_ty: hir::Ty) -> hir::TypedExpr {
    let n_pop_native = hir::Expr::raw_i64(n_pop as i64);
    let env_pop = {
        let fun_ty = hir::FunTy {
            asyncness: hir::Asyncness::Lowered,
            param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Int64],
            ret_ty: Box::new(hir::Ty::Any),
        };
        let fname = FunctionName::mangled("chiika_env_pop_frame");
        hir::Expr::func_ref(fname, fun_ty)
    };
    let cast_type = match &popped_value_ty {
        hir::Ty::Int => hir::CastType::AnyToInt,
        hir::Ty::Fun(fun_ty) => hir::CastType::AnyToFun(fun_ty.clone()),
        _ => panic!("[BUG] cannot cast: {:?}", popped_value_ty),
    };
    hir::Expr::cast(
        cast_type,
        hir::Expr::fun_call(env_pop, vec![arg_ref_env(), n_pop_native]),
    )
}

fn call_chiika_spawn(f: hir::TypedExpr) -> hir::TypedExpr {
    let null_cont_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Void],
        ret_ty: Box::new(hir::Ty::RustFuture),
    };
    let new_f_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, null_cont_ty.into()],
        ret_ty: Box::new(hir::Ty::RustFuture),
    };
    let new_f = (f.0, new_f_ty.clone().into());
    let fun_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::Fun(new_f_ty)],
        ret_ty: Box::new(hir::Ty::Void),
    };
    let fname = FunctionName::mangled("chiika_spawn");
    hir::Expr::fun_call(hir::Expr::func_ref(fname, fun_ty), vec![new_f])
}

#[derive(Debug)]
struct Chapters {
    chaps: Vec<Chapter>,
}

impl Chapters {
    fn new() -> Chapters {
        Chapters { chaps: vec![] }
    }

    fn extract(&mut self) -> VecDeque<Chapter> {
        self.chaps.drain(..).collect()
    }

    fn len(&self) -> usize {
        self.chaps.len()
    }

    fn last_mut(&mut self) -> &mut Chapter {
        self.chaps.last_mut().unwrap()
    }

    /// Returns the name of the last chapter
    fn current_name(&self) -> &str {
        &self.chaps.last().unwrap().name
    }

    fn add(&mut self, chap: Chapter) {
        self.chaps.push(chap);
    }

    fn add_stmt(&mut self, stmt: hir::TypedExpr) {
        self.chaps.last_mut().unwrap().add_stmt(stmt);
    }

    fn add_stmts(&mut self, stmts: Vec<hir::TypedExpr>) {
        self.chaps.last_mut().unwrap().add_stmts(stmts);
    }
}

#[derive(Debug)]
struct Chapter {
    stmts: Vec<hir::TypedExpr>,
    // The resulting type of the async function called with the last stmt
    async_result_ty: Option<hir::Ty>,
    name: String,
    params: Vec<hir::Param>,
    ret_ty: hir::Ty,
}

impl Chapter {
    fn new_original(f: &hir::Function) -> Chapter {
        if f.asyncness.is_async() {
            let async_result_ty = f.ret_ty.clone();
            let mut params = f.params.clone();
            params.push(hir::Param::new(
                hir::Ty::Fun(hir::FunTy {
                    asyncness: hir::Asyncness::Lowered,
                    param_tys: vec![hir::Ty::ChiikaEnv, async_result_ty],
                    ret_ty: Box::new(hir::Ty::RustFuture),
                }),
                "$cont",
            ));
            Self::new(f.name.to_string(), params, hir::Ty::RustFuture)
        } else {
            Self::new(f.name.to_string(), f.params.clone(), f.ret_ty.clone())
        }
    }

    fn new_async_if_clause(name: String, suffix: &str) -> Chapter {
        let params = vec![hir::Param::new(hir::Ty::ChiikaEnv, "$env")];
        Self::new(format!("{}'{}", name, suffix), params, hir::Ty::RustFuture)
    }

    fn new_async_end_if(name: String, suffix: &str, if_ty: hir::Ty) -> Chapter {
        let params = vec![
            hir::Param::new(hir::Ty::ChiikaEnv, "$env"),
            hir::Param::new(if_ty, "$ifResult"),
        ];
        Self::new(format!("{}'{}", name, suffix), params, hir::Ty::RustFuture)
    }

    fn new_async_call_receiver(name: FunctionName, async_result_ty: hir::Ty) -> Chapter {
        let params = vec![
            hir::Param::new(hir::Ty::ChiikaEnv, "$env"),
            hir::Param::new(async_result_ty.clone(), "$async_result"),
        ];
        Self::new(name.to_string(), params, hir::Ty::RustFuture)
    }

    fn new(name: String, params: Vec<hir::Param>, ret_ty: hir::Ty) -> Chapter {
        Chapter {
            stmts: vec![],
            async_result_ty: None,
            name,
            params,
            ret_ty,
        }
    }

    fn add_stmt(&mut self, stmt: hir::TypedExpr) {
        self.stmts.push(stmt);
    }

    fn add_stmts(&mut self, stmts: Vec<hir::TypedExpr>) {
        self.stmts.extend(stmts);
    }
}
