//! Example
//! ```
//! // Before
//! fun foo() -> Int {
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
use anyhow::{anyhow, Result};
use std::collections::VecDeque;

/// Splits asynchronous Milika func into multiple funcs.
/// Also, signatures of async externs are modified to take `$env` and `$cont` as the first two params.
pub fn run(hir: hir::Program) -> Result<hir::Program> {
    let externs = hir
        .externs
        .into_iter()
        .map(|e| {
            if e.is_async {
                hir::Extern {
                    params: append_async_params(&e.params, e.ret_ty.clone(), false),
                    ret_ty: hir::Ty::RustFuture,
                    ..e
                }
            } else {
                e
            }
        })
        .collect();

    let mut funcs = vec![];
    for mut f in hir.funcs {
        let allocs = hir::visitor::Allocs::collect(&f.body_stmts)?;
        let mut c = Compiler {
            orig_func: &mut f,
            allocs,
            chapters: Chapters::new(),
            gensym_ct: 0,
        };
        let mut split_funcs = c.compile_func()?;
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
    fn compile_func(&mut self) -> Result<Vec<hir::Function>> {
        self.chapters.add(Chapter::new_original(self.orig_func));
        if self.orig_func.asyncness.is_async() {
            self._compile_async_intro();
        }
        for expr in self.orig_func.body_stmts.drain(..).collect::<Vec<_>>() {
            if let Some(new_expr) = self.compile_expr(expr, false)? {
                self.chapters.add_stmt(new_expr);
            }
        }

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
        let mut push_items = vec![arg_ref_cont(arity, self.orig_func.ret_ty.clone())];
        for i in 0..arity {
            push_items.push(hir::Expr::arg_ref(
                i + 1, // +1 for $env
                self.orig_func.params[i].ty.clone(),
            ));
        }
        for (i, arg) in push_items.into_iter().enumerate() {
            self.chapters.add_stmt(call_chiika_env_set(i, arg));
        }
    }

    fn _serialize_chapter(&self, chap: Chapter) -> hir::Function {
        let f = hir::Function {
            generated: self.orig_func.generated,
            asyncness: hir::Asyncness::Lowered,
            name: chap.name,
            params: chap.params,
            ret_ty: chap.ret_ty,
            body_stmts: chap.stmts,
        };
        match chap.chaptype {
            ChapterType::Original => f,
            ChapterType::AsyncIfClause => f,
            ChapterType::AsyncEndIf => f,
            ChapterType::AsyncCallReceiver => f,
        }
    }

    fn compile_value_expr(&mut self, e: hir::TypedExpr, on_return: bool) -> Result<hir::TypedExpr> {
        if let Some(expr) = self.compile_expr(e, on_return)? {
            Ok(expr)
        } else {
            Err(anyhow!("[BUG] unexpected void expr"))
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
            hir::Expr::LVarRef(ref varname) => {
                let i = self.lvar_idx(varname);
                call_chiika_env_ref(hir::Expr::number(i as i64))
            }
            hir::Expr::ArgRef(idx) => {
                if self.chapters.len() == 1 {
                    let i = if self.orig_func.asyncness.is_async() {
                        idx + 1
                    } else {
                        idx
                    };
                    hir::Expr::arg_ref(i, e.1)
                } else {
                    let i = idx + 1; // +1 for $cont
                    call_chiika_env_ref(hir::Expr::number(i as i64))
                }
            }
            hir::Expr::FuncRef(_) => e,
            hir::Expr::OpCall(op, lhs, rhs) => {
                let l = self.compile_value_expr(*lhs, false)?;
                let r = self.compile_value_expr(*rhs, false)?;
                hir::Expr::op_call(op, l, r)
            }
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
            hir::Expr::Assign(varname, rhs) => {
                let i = self.lvar_idx(&varname);
                let v = self.compile_value_expr(*rhs, false)?;
                call_chiika_env_set(i, v)
            }
            hir::Expr::If(cond_expr, then_exprs, else_exprs) => {
                return self.compile_if(&e.1, *cond_expr, then_exprs, else_exprs);
            }
            hir::Expr::Yield(expr) => {
                let new_expr = self.compile_value_expr(*expr, false)?;
                hir::Expr::yield_(new_expr)
            }
            hir::Expr::While(_cond_expr, _body_exprs) => todo!(),
            hir::Expr::Spawn(fexpr) => {
                let new_fexpr = self.compile_value_expr(*fexpr, false)?;
                call_chiika_spawn(new_fexpr)
            }
            hir::Expr::Alloc(_) => hir::Expr::nop(),
            hir::Expr::Return(expr) => self.compile_return(*expr)?,
            _ => panic!("[BUG] unexpected for async_splitter: {:?}", e.0),
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

    fn compile_if(
        &mut self,
        if_ty: &hir::Ty,
        cond_expr: hir::TypedExpr,
        then_exprs: Vec<hir::TypedExpr>,
        else_exprs: Vec<hir::TypedExpr>,
    ) -> Result<Option<hir::TypedExpr>> {
        let new_cond_expr = self.compile_value_expr(cond_expr, false)?;
        if self.orig_func.asyncness.is_sync() {
            let then = self.compile_exprs(then_exprs)?;
            let els = self.compile_exprs(else_exprs)?;
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
            vec![hir::Expr::return_(fcall_t)],
            vec![hir::Expr::return_(fcall_f)],
        );
        self.chapters.add_stmt(terminator);

        self.chapters.add(then_chap);
        self.compile_if_clause(then_exprs, &endif_chap.name)?;
        self.chapters.add(else_chap);
        self.compile_if_clause(else_exprs, &endif_chap.name)?;

        if *if_ty == hir::Ty::Void {
            // Both branches end with return
            Ok(None)
        } else {
            self.chapters.add(endif_chap);
            // FIXME: This magic number is decided by async_splitter.rs
            Ok(Some(hir::Expr::arg_ref(1, if_ty.clone())))
        }
    }

    fn compile_exprs(&mut self, exprs: Vec<hir::TypedExpr>) -> Result<Vec<hir::TypedExpr>> {
        debug_assert!(self.orig_func.asyncness.is_sync());
        let mut new_exprs = vec![];
        for expr in exprs {
            if let Some(new_expr) = self.compile_expr(expr, false)? {
                new_exprs.push(new_expr);
            }
        }
        Ok(new_exprs)
    }

    fn compile_if_clause(
        &mut self,
        mut exprs: Vec<hir::TypedExpr>,
        endif_chap_name: &str,
    ) -> Result<()> {
        let e = exprs.pop().unwrap();
        let opt_vexpr = match e {
            (hir::Expr::Return(_), _) => {
                exprs.push(e);
                None
            }
            (hir::Expr::Yield(vexpr), _) => Some(vexpr),
            _ => {
                return Err(anyhow!(
                    "[BUG] last statement of a clause must be a yield or a return"
                ))
            }
        };
        for expr in exprs {
            if let Some(new_expr) = self.compile_expr(expr, false)? {
                self.chapters.add_stmt(new_expr);
            }
        }
        if let Some(vexpr) = opt_vexpr {
            let new_vexpr = self.compile_value_expr(*vexpr, false)?;
            let goto_endif = hir::Expr::fun_call(
                hir::Expr::func_ref(
                    endif_chap_name,
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
        // TODO: Support local variables
        let args = vec![arg_ref_env()];
        let chap_fun_ty = hir::FunTy {
            asyncness: self.orig_func.asyncness.clone(),
            param_tys: vec![hir::Ty::ChiikaEnv],
            ret_ty: Box::new(hir::Ty::RustFuture),
        };
        hir::Expr::fun_call(hir::Expr::func_ref(chap_name, chap_fun_ty), args)
    }

    fn compile_return(&mut self, expr: hir::TypedExpr) -> Result<hir::TypedExpr> {
        let new_expr = self.compile_value_expr(expr, true)?;
        if self.orig_func.asyncness.is_sync() {
            return Ok(hir::Expr::return_(new_expr));
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
            // Convert `callee(args...)`
            // to `callee(args..., env, env_pop())`
            let hir::Expr::FunCall(fexpr, mut args) = new_expr.0 else {
                unreachable!();
            };
            args.insert(0, arg_ref_env());
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
        Ok(hir::Expr::return_(value_expr))
    }

    fn lvar_idx(&self, varname: &str) -> usize {
        let i = self
            .allocs
            .iter()
            .position(|(name, _)| name == varname)
            .expect("[BUG] lvar not in self.lvars");
        // +1 for $cont
        1 + self.orig_func.params.len() + i
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

// Convert `Fun((X)->Y)` to `Fun((ChiikaEnv, X, Fun((Y,ChiikaEnv)->RustFuture))->RustFuture)`
fn async_fun_ty(orig_fun_ty: &hir::FunTy) -> hir::FunTy {
    let mut param_tys = orig_fun_ty.param_tys.clone();
    param_tys.insert(0, hir::Ty::ChiikaEnv);
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
    next_chapter_name: String,
) -> hir::TypedExpr {
    let hir::Ty::Fun(fun_ty) = &fexpr.1 else {
        panic!("[BUG] not a function: {:?}", fexpr.0);
    };
    // Append `$env` and `$cont` (i.e. the next chapter)
    args.insert(0, arg_ref_env());
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

/// Append params for async (`$env` and `$cont`)
fn append_async_params(
    params: &[hir::Param],
    result_ty: hir::Ty,
    generated: bool,
) -> Vec<hir::Param> {
    let mut new_params = params.to_vec();
    if generated {
        new_params.insert(0, hir::Param::new(hir::Ty::ChiikaEnv, "$env"));
    } else {
        new_params.insert(0, hir::Param::new(hir::Ty::ChiikaEnv, "$env"));
        let fun_ty = hir::FunTy {
            asyncness: hir::Asyncness::Lowered,
            param_tys: vec![hir::Ty::ChiikaEnv, result_ty],
            ret_ty: Box::new(hir::Ty::RustFuture),
        };
        new_params.push(hir::Param::new(hir::Ty::Fun(fun_ty), "$cont"));
    }

    new_params
}

/// Create name of generated function like `foo_1`
fn chapter_func_name(orig_name: &str, chapter_idx: usize) -> String {
    format!("{}_{}", orig_name, chapter_idx)
}

/// Get the `$env` that is 0-th param of async func
fn arg_ref_env() -> hir::TypedExpr {
    hir::Expr::arg_ref(0, hir::Ty::ChiikaEnv)
}

/// Get the `$cont` param of async func
/// The continuation takes an argument.
fn arg_ref_cont(arity: usize, arg_ty: hir::Ty) -> hir::TypedExpr {
    let cont_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, arg_ty],
        ret_ty: Box::new(hir::Ty::RustFuture),
    };
    hir::Expr::arg_ref(arity + 1, hir::Ty::Fun(cont_ty))
}

/// Get the `$async_result` which is 1-th param of chapter func
fn arg_ref_async_result(ty: hir::Ty) -> hir::TypedExpr {
    hir::Expr::arg_ref(1, ty)
}

fn call_chiika_env_push_frame(size: usize) -> hir::TypedExpr {
    hir::Expr::fun_call(
        hir::Expr::func_ref(
            "chiika_env_push_frame",
            hir::FunTy {
                asyncness: hir::Asyncness::Lowered,
                param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Int],
                ret_ty: Box::new(hir::Ty::Null),
            },
        ),
        vec![arg_ref_env(), hir::Expr::number(size as i64)],
    )
}

fn call_chiika_env_pop_frame(n_pop: usize, popped_value_ty: hir::Ty) -> hir::TypedExpr {
    let env_pop = {
        let fun_ty = hir::FunTy {
            asyncness: hir::Asyncness::Lowered,
            param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Int],
            ret_ty: Box::new(hir::Ty::Any),
        };
        hir::Expr::func_ref("chiika_env_pop_frame", fun_ty)
    };
    let cast_type = match &popped_value_ty {
        hir::Ty::Int => hir::CastType::AnyToInt,
        hir::Ty::Fun(fun_ty) => hir::CastType::AnyToFun(fun_ty.clone()),
        _ => panic!("[BUG] cannot cast: {:?}", popped_value_ty),
    };
    hir::Expr::cast(
        cast_type,
        hir::Expr::fun_call(
            env_pop,
            vec![arg_ref_env(), hir::Expr::number(n_pop as i64)],
        ),
    )
}

fn call_chiika_env_set(i: usize, val: hir::TypedExpr) -> hir::TypedExpr {
    let idx = hir::Expr::number(i as i64);
    let type_id = hir::Expr::number(val.1.type_id());
    let cast_val = {
        let cast_type = match val.1 {
            hir::Ty::Null => hir::CastType::NullToAny,
            hir::Ty::Int => hir::CastType::IntToAny,
            hir::Ty::Fun(_) => hir::CastType::FunToAny,
            _ => panic!("[BUG] don't know how to cast {:?} to Any", val),
        };
        hir::Expr::cast(cast_type, val)
    };
    let fun_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Int, hir::Ty::Any, hir::Ty::Int],
        ret_ty: Box::new(hir::Ty::Null),
    };
    hir::Expr::fun_call(
        hir::Expr::func_ref("chiika_env_set", fun_ty),
        vec![arg_ref_env(), idx, cast_val, type_id],
    )
}

fn call_chiika_env_ref(n: hir::TypedExpr) -> hir::TypedExpr {
    let type_id = hir::Expr::number(hir::Ty::Int.type_id());
    let fun_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Int, hir::Ty::Int],
        // Milika lvars are all int
        ret_ty: Box::new(hir::Ty::Int),
    };
    hir::Expr::fun_call(
        hir::Expr::func_ref("chiika_env_ref", fun_ty),
        vec![arg_ref_env(), n, type_id],
    )
}

fn call_chiika_spawn(f: hir::TypedExpr) -> hir::TypedExpr {
    let null_cont_ty = hir::FunTy {
        asyncness: hir::Asyncness::Lowered,
        param_tys: vec![hir::Ty::ChiikaEnv, hir::Ty::Null],
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
        ret_ty: Box::new(hir::Ty::Null),
    };
    hir::Expr::fun_call(hir::Expr::func_ref("chiika_spawn", fun_ty), vec![new_f])
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
    chaptype: ChapterType,
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
            params.insert(0, hir::Param::new(hir::Ty::ChiikaEnv, "$env"));
            params.push(hir::Param::new(
                hir::Ty::Fun(hir::FunTy {
                    asyncness: hir::Asyncness::Lowered,
                    param_tys: vec![hir::Ty::ChiikaEnv, async_result_ty],
                    ret_ty: Box::new(hir::Ty::RustFuture),
                }),
                "$cont",
            ));
            Self::new(
                ChapterType::Original,
                f.name.clone(),
                params,
                hir::Ty::RustFuture,
            )
        } else {
            Self::new(
                ChapterType::Original,
                f.name.clone(),
                f.params.clone(),
                f.ret_ty.clone(),
            )
        }
    }

    fn new_async_if_clause(name: String, suffix: &str) -> Chapter {
        let params = vec![hir::Param::new(hir::Ty::ChiikaEnv, "$env")];
        Self::new(
            ChapterType::AsyncIfClause,
            format!("{}'{}", name, suffix),
            params,
            hir::Ty::RustFuture,
        )
    }

    fn new_async_end_if(name: String, suffix: &str, if_ty: hir::Ty) -> Chapter {
        let params = vec![
            hir::Param::new(hir::Ty::ChiikaEnv, "$env"),
            hir::Param::new(if_ty, "$ifResult"),
        ];
        Self::new(
            ChapterType::AsyncEndIf,
            format!("{}'{}", name, suffix),
            params,
            hir::Ty::RustFuture,
        )
    }

    fn new_async_call_receiver(name: String, async_result_ty: hir::Ty) -> Chapter {
        let params = vec![
            hir::Param::new(hir::Ty::ChiikaEnv, "$env"),
            hir::Param::new(async_result_ty.clone(), "$async_result"),
        ];
        Self::new(
            ChapterType::AsyncCallReceiver,
            name,
            params,
            hir::Ty::RustFuture,
        )
    }

    fn new(
        chaptype: ChapterType,
        name: String,
        params: Vec<hir::Param>,
        ret_ty: hir::Ty,
    ) -> Chapter {
        Chapter {
            chaptype,
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

#[derive(Debug)]
enum ChapterType {
    Original,
    AsyncIfClause,
    AsyncEndIf,
    AsyncCallReceiver,
}
