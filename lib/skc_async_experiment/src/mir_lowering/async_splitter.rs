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
use crate::mir;
use crate::names::FunctionName;
use anyhow::Result;
use skc_hir::MethodSignature;
use std::collections::VecDeque;

/// Splits asynchronous Milika func into multiple funcs.
/// Also, signatures of async externs are modified to take `$env` and `$cont` as the first two params.
pub fn run(mir: mir::Program) -> Result<mir::Program> {
    let externs = mir
        .externs
        .into_iter()
        .map(|e| {
            if e.is_async() {
                mir::Extern {
                    fun_ty: mir::FunTy::lowered(
                        append_async_params(&e.fun_ty),
                        mir::Ty::RustFuture,
                    ),
                    ..e
                }
            } else {
                e
            }
        })
        .collect();

    let mut funcs = vec![];
    for mut f in mir.funcs {
        if f.asyncness.is_sync() {
            funcs.push(f);
            continue;
        }
        let allocs = mir::visitor::Allocs::collect(&f.body_stmts);
        // Extract body_stmts from f
        let mut body_stmts = mir::Expr::nop();
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
    Ok(mir::Program::new(
        mir.classes,
        externs,
        funcs,
        mir.constants,
    ))
}

#[derive(Debug)]
struct Compiler<'a> {
    orig_func: &'a mut mir::Function,
    allocs: Vec<(String, mir::Ty)>,
    chapters: Chapters,
    gensym_ct: usize,
}

impl<'a> Compiler<'a> {
    /// Entry point for each milika function
    fn compile_func(&mut self, body_stmts: mir::TypedExpr) -> Result<Vec<mir::Function>> {
        self.chapters.add(Chapter::new_original(self.orig_func));
        if self.orig_func.asyncness.is_async() {
            self._compile_async_intro();
        }
        self.compile_stmts(mir::expr::into_exprs(body_stmts))?;
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
        // Store $cont to env[0]
        self.chapters.add_stmt(mir::Expr::env_set(
            0,
            arg_ref_cont(arity, self.orig_func.ret_ty.clone()),
            "$cont",
        ));
        // Store args to env[1..]
        for i in 1..arity {
            let param = &self.orig_func.params[i];
            self.chapters.add_stmt(mir::Expr::env_set(
                i,
                mir::Expr::arg_ref(i, &param.name, param.ty.clone()),
                &param.name,
            ));
        }
    }

    fn _serialize_chapter(&self, chap: Chapter) -> mir::Function {
        mir::Function {
            asyncness: mir::Asyncness::Lowered,
            sig: chap.sig,
            name: chap.name.clone(),
            params: chap.params,
            ret_ty: chap.ret_ty,
            body_stmts: mir::Expr::exprs(chap.stmts),
        }
    }

    fn compile_value_expr(&mut self, e: mir::TypedExpr, on_return: bool) -> Result<mir::TypedExpr> {
        let tmp = e.clone();
        if let Some(expr) = self.compile_expr(e, on_return)? {
            Ok(expr)
        } else {
            println!("{:?}", &tmp);
            panic!("Got None in compile_value_expr (async call?)")
        }
    }

    /// Examine each expression for special care. Most important part is
    /// calling async function.
    fn compile_expr(
        &mut self,
        e: mir::TypedExpr,
        on_return: bool,
    ) -> Result<Option<mir::TypedExpr>> {
        let new_e = match e.0 {
            mir::Expr::Number(_) => e,
            mir::Expr::PseudoVar(_) => e,
            mir::Expr::LVarRef(_) => panic!("LVarRef must be lowered to EnvRef"),
            mir::Expr::IVarRef(obj_expr, idx, name) => {
                let new_obj = self.compile_value_expr(*obj_expr, false)?;
                mir::Expr::ivar_ref(new_obj, idx, name, e.1.clone())
            }
            mir::Expr::ArgRef(_, _) => e,
            mir::Expr::EnvRef(_, _) => e,
            mir::Expr::EnvSet(idx, rhs, name) => {
                let v = self.compile_value_expr(*rhs, false)?;
                mir::Expr::env_set(idx, v, name)
            }
            mir::Expr::ConstRef(_) => e,
            mir::Expr::FuncRef(_) => e,
            mir::Expr::FunCall(fexpr, arg_exprs) => {
                let new_fexpr = self.compile_value_expr(*fexpr, false)?;
                let new_args = arg_exprs
                    .into_iter()
                    .map(|x| self.compile_value_expr(x, false))
                    .collect::<Result<Vec<_>>>()?;
                let fun_ty = new_fexpr.1.as_fun_ty();
                if fun_ty.asyncness.is_async() && !on_return {
                    self.compile_async_call(new_fexpr, new_args, e.1)?
                } else {
                    // No need to create a new chapter if on_return is true.
                    // In that case the args are modified later (see compile_return)
                    mir::Expr::fun_call(new_fexpr, new_args)
                }
            }
            mir::Expr::VTableRef(receiver, idx, name) => {
                let new_receiver = self.compile_value_expr(*receiver, false)?;
                mir::Expr::vtable_ref(new_receiver, idx, name, e.1.into_fun_ty())
            }
            mir::Expr::WTableRef(receiver, module, idx, name) => {
                let new_receiver = self.compile_value_expr(*receiver, false)?;
                mir::Expr::wtable_ref(new_receiver, module, idx, name, e.1.into_fun_ty())
            }
            mir::Expr::If(cond_expr, then_exprs, else_exprs) => {
                return self.compile_if(&e.1, *cond_expr, *then_exprs, *else_exprs);
            }
            mir::Expr::While(cond_expr, body_exprs) => {
                return self.compile_while(*cond_expr, *body_exprs);
            }
            mir::Expr::Spawn(fexpr) => {
                let new_fexpr = self.compile_value_expr(*fexpr, false)?;
                call_chiika_spawn(new_fexpr)
            }
            mir::Expr::Alloc(_, _) => mir::Expr::nop(),
            mir::Expr::LVarSet(_, _) => {
                panic!("LVarSet must be lowered to EnvSet");
            }
            mir::Expr::IVarSet(obj, idx, rhs, name) => {
                let new_obj = self.compile_value_expr(*obj, false)?;
                let new_rhs = self.compile_value_expr(*rhs, false)?;
                mir::Expr::ivar_set(new_obj, idx, new_rhs, name)
            }
            mir::Expr::ConstSet(name, rhs) => {
                let v = self.compile_value_expr(*rhs, false)?;
                mir::Expr::const_set(name, v)
            }
            mir::Expr::Return(expr) => return self.compile_return(*expr),
            mir::Expr::Exprs(_) => {
                panic!("Exprs must be handled by its parent: {:?}", e.0);
            }
            mir::Expr::Cast(cast_type, expr) => {
                let new_expr = self.compile_value_expr(*expr, on_return)?;
                mir::Expr::cast(cast_type, new_expr)
            }
            mir::Expr::CreateObject(_) => e,
            mir::Expr::CreateTypeObject(_) => e,
            mir::Expr::StringLiteral(_) => e,
            mir::Expr::CreateNativeArray(elem_exprs) => {
                // TODO: async in array elements
                let new_elems = elem_exprs
                    .into_iter()
                    .map(|elem| self.compile_value_expr(elem, false))
                    .collect::<Result<Vec<_>>>()?;
                (mir::Expr::CreateNativeArray(new_elems), e.1.clone())
            }
            mir::Expr::Unbox(_) | mir::Expr::RawI64(_) | mir::Expr::Nop => e,
            mir::Expr::WTableKey(_) | mir::Expr::WTableRow(_, _) => e,
        };
        Ok(Some(new_e))
    }

    /// On calling an async function, create a new chapter and
    /// append the async call to the current chapter
    fn compile_async_call(
        &mut self,
        fexpr: mir::TypedExpr,
        args: Vec<mir::TypedExpr>,
        async_result_ty: mir::Ty,
    ) -> Result<mir::TypedExpr> {
        // Change chapter here
        let next_chapter_name = chapter_func_name(&self.orig_func.name, self.chapters.len());
        let last_chapter = self.chapters.last_mut();
        let terminator = mir::Expr::return_(modify_async_call(fexpr, args, next_chapter_name));
        last_chapter.stmts.push(terminator);
        last_chapter.async_result_ty = Some(async_result_ty.clone());
        self.chapters.add(Chapter::new_async_call_receiver(
            chapter_func_name(&self.orig_func.name, self.chapters.len()),
            async_result_ty.clone(),
        ));

        Ok(arg_ref_async_result(async_result_ty))
    }

    /// Compile a list of statements. It may contain an async call.
    fn compile_stmts(&mut self, stmts: Vec<mir::TypedExpr>) -> Result<()> {
        for stmt in stmts {
            if let Some(new_stmt) = self.compile_expr(stmt, false)? {
                self.chapters.add_stmt(new_stmt);
            }
        }
        Ok(())
    }

    fn compile_if(
        &mut self,
        if_ty: &mir::Ty,
        cond_expr: mir::TypedExpr,
        then_exprs: mir::TypedExpr,
        else_exprs_: mir::TypedExpr,
    ) -> Result<Option<mir::TypedExpr>> {
        let new_cond_expr = self.compile_value_expr(cond_expr, false)?;
        let func_name = self.chapters.current_name();

        let then_chap = Chapter::new_async_if_clause(func_name.clone(), "t");
        let else_chap = Chapter::new_async_if_clause(func_name.clone(), "f");
        // Statements after `if` goes to an "endif" chapter
        let endif_chap = Chapter::new_async_end_if(func_name.clone(), "e", if_ty.clone()); // e for endif

        let fcall_t = self.branch_call(&then_chap.name);
        let fcall_f = self.branch_call(&else_chap.name);
        let terminator = mir::Expr::if_(
            new_cond_expr,
            mir::Expr::return_(fcall_t),
            mir::Expr::return_(fcall_f),
        );
        self.chapters.add_stmt(terminator);

        self.chapters.add(then_chap);
        self.compile_if_clause(then_exprs, &endif_chap.name)?;
        self.chapters.add(else_chap);
        self.compile_if_clause(else_exprs_, &endif_chap.name)?;

        if *if_ty == mir::Ty::raw("Never") {
            // Both branches end with return
            Ok(None)
        } else {
            self.chapters.add(endif_chap);
            Ok(Some(mir::Expr::arg_ref(1, "$ifResult", if_ty.clone())))
        }
    }

    fn compile_if_clause(
        &mut self,
        exprs_: mir::TypedExpr,
        endif_chap_name: &FunctionName,
    ) -> Result<()> {
        let mut exprs = mir::expr::into_exprs(exprs_);
        let e = exprs.pop().unwrap();
        let opt_vexpr = if e.1 == mir::Ty::raw("Never") {
            exprs.push(e);
            None
        } else {
            Some(e)
        };
        self.compile_stmts(exprs)?;
        // Send the value to the endif chapter (unless ends with `return`)
        if let Some(vexpr) = opt_vexpr {
            let new_vexpr = self.compile_value_expr(vexpr, false)?;
            let goto_endif = mir::Expr::fun_call(
                mir::Expr::func_ref(
                    endif_chap_name.clone(),
                    mir::FunTy {
                        asyncness: mir::Asyncness::Lowered,
                        param_tys: vec![mir::Ty::ChiikaEnv, new_vexpr.1.clone()],
                        ret_ty: Box::new(mir::Ty::RustFuture),
                    },
                ),
                vec![arg_ref_env(), new_vexpr],
            );
            self.chapters.add_stmt(mir::Expr::return_(goto_endif));
        }
        Ok(())
    }

    /// Generate a call to the if-branch function
    fn branch_call(&self, chap_name: &FunctionName) -> mir::TypedExpr {
        let args = vec![arg_ref_env()];
        let chap_fun_ty = mir::FunTy {
            asyncness: self.orig_func.asyncness.clone(),
            param_tys: vec![mir::Ty::ChiikaEnv],
            ret_ty: Box::new(mir::Ty::RustFuture),
        };
        mir::Expr::fun_call(mir::Expr::func_ref(chap_name.clone(), chap_fun_ty), args)
    }

    fn compile_while(
        &mut self,
        cond_expr: mir::TypedExpr,
        body_expr: mir::TypedExpr,
    ) -> Result<Option<mir::TypedExpr>> {
        let func_name = self.chapters.current_name();

        let beginwhile_chap = Chapter::new_beginwhile_clause(func_name);
        let jump_to_beginwhile = self.while_jump(&beginwhile_chap.name);
        let whilebody_chap = Chapter::new_whilebody_clause(func_name);
        let endwhile_chap = Chapter::new_endwhile_clause(func_name);

        self.chapters.add_stmt(jump_to_beginwhile.clone());
        // Create beginwhile chapter
        self.chapters.add(beginwhile_chap);
        let new_cond_expr = self.compile_value_expr(cond_expr, false)?;
        self.chapters.add_stmt(mir::Expr::if_(
            new_cond_expr,
            self.while_jump(&whilebody_chap.name),
            self.while_jump(&endwhile_chap.name),
        ));

        // Create whilebody chapter
        self.chapters.add(whilebody_chap);
        self.compile_stmts(mir::expr::into_exprs(body_expr))?;
        self.chapters.add_stmt(jump_to_beginwhile);

        // Create endwhile chapter
        self.chapters.add(endwhile_chap);
        Ok(None)
    }

    fn while_jump(&self, chap_name: &FunctionName) -> mir::TypedExpr {
        let chap_func_ty = mir::Ty::Fun(mir::FunTy {
            asyncness: mir::Asyncness::Async,
            param_tys: vec![mir::Ty::ChiikaEnv],
            ret_ty: Box::new(mir::Ty::RustFuture),
        });
        mir::Expr::return_(mir::Expr::fun_call(
            mir::Expr::func_ref(chap_name.clone(), chap_func_ty.into()),
            vec![mir::Expr::arg_ref(0, "$env", mir::Ty::ChiikaEnv)],
        ))
    }

    fn compile_return(&mut self, expr: mir::TypedExpr) -> Result<Option<mir::TypedExpr>> {
        // `return return 1` == `return 1`
        if expr.1 == mir::Ty::raw("Never") {
            return self.compile_expr(expr, false);
        }
        let new_expr = self.compile_value_expr(expr, true)?;
        if self.orig_func.asyncness.is_sync() {
            return Ok(Some(mir::Expr::return_(new_expr)));
        }
        let env_pop = {
            let cont_ty = mir::Ty::Fun(mir::FunTy {
                asyncness: mir::Asyncness::Lowered,
                param_tys: vec![mir::Ty::ChiikaEnv, self.orig_func.ret_ty.clone()],
                ret_ty: Box::new(mir::Ty::RustFuture),
            });
            call_chiika_env_pop_frame(self.frame_size(), cont_ty)
        };
        let ret = match new_expr {
            (mir::Expr::FunCall(fexpr, args), _) if fexpr.1.is_async_fun().unwrap() => {
                self.continue_with_func(*fexpr, args, env_pop)
            }
            _ => self.continue_with_value(new_expr, env_pop),
        };
        Ok(Some(ret))
    }

    // Call the async function and pass the result to the continuation.
    // Convert `callee($env, args...)` to
    // `callee($env, args..., env_pop())`
    fn continue_with_func(
        &mut self,
        fexpr: mir::TypedExpr,
        mut args: Vec<mir::TypedExpr>,
        env_pop: mir::TypedExpr,
    ) -> mir::TypedExpr {
        args.push(env_pop);
        let new_fexpr = (fexpr.0, async_fun_ty(fexpr.1.as_fun_ty()).into());
        mir::Expr::return_(mir::Expr::fun_call(new_fexpr, args))
    }

    // Pass the value to the continuation function. Example:
    //   alloc tmp;    # tmp is needed because calculating value may call env_ref
    //   tmp = value;
    //   `(env_pop())(env, tmp)`;
    fn continue_with_value(
        &mut self,
        new_expr: mir::TypedExpr,
        env_pop: mir::TypedExpr,
    ) -> mir::TypedExpr {
        let tmp = self.store_to_tmpvar(new_expr);
        mir::Expr::return_(mir::Expr::fun_call(env_pop, vec![arg_ref_env(), tmp]))
    }

    /// Store the value to a temporary variable and return the varref
    fn store_to_tmpvar(&mut self, value: mir::TypedExpr) -> mir::TypedExpr {
        let ty = value.1.clone();
        let varname = self.gensym();
        self.chapters.add_stmts(vec![
            mir::Expr::alloc(varname.clone(), value.1.clone()),
            mir::Expr::lvar_set(varname.clone(), value),
        ]);
        mir::Expr::lvar_ref(varname, ty)
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
fn async_fun_ty(orig_fun_ty: &mir::FunTy) -> mir::FunTy {
    let mut param_tys = orig_fun_ty.param_tys.clone();
    let orig_ret_ty = orig_fun_ty.ret_ty.as_ref().clone();
    param_tys.push(mir::Ty::Fun(mir::FunTy {
        asyncness: mir::Asyncness::Lowered,
        param_tys: vec![mir::Ty::ChiikaEnv, orig_ret_ty],
        ret_ty: Box::new(mir::Ty::RustFuture),
    }));
    mir::FunTy {
        asyncness: mir::Asyncness::Async,
        param_tys,
        ret_ty: Box::new(mir::Ty::RustFuture),
    }
}

fn modify_async_call(
    fexpr: mir::TypedExpr,
    mut args: Vec<mir::TypedExpr>,
    next_chapter_name: FunctionName,
) -> mir::TypedExpr {
    let mir::Ty::Fun(fun_ty) = &fexpr.1 else {
        panic!("[BUG] not a function: {:?}", fexpr.0);
    };
    // Append `$cont` (i.e. the next chapter)
    let next_chapter = {
        let next_chapter_ty = mir::FunTy {
            asyncness: mir::Asyncness::Lowered,
            param_tys: vec![mir::Ty::ChiikaEnv, *fun_ty.ret_ty.clone()],
            ret_ty: Box::new(mir::Ty::RustFuture),
        };
        mir::Expr::func_ref(next_chapter_name, next_chapter_ty)
    };
    args.push(next_chapter);
    let new_fexpr = (fexpr.0, async_fun_ty(fexpr.1.as_fun_ty()).into());
    mir::Expr::fun_call(new_fexpr, args)
}

/// Append param for async libfunc (i.e. `$cont`)
fn append_async_params(fun_ty: &mir::FunTy) -> Vec<mir::Ty> {
    let mut new_params = fun_ty.param_tys.to_vec();
    let cont_ty = mir::FunTy::lowered(
        vec![mir::Ty::ChiikaEnv, *fun_ty.ret_ty.clone()],
        mir::Ty::RustFuture,
    );
    new_params.push(mir::Ty::Fun(cont_ty));
    new_params
}

/// Create name of generated function like `foo_1`
fn chapter_func_name(orig_name: &FunctionName, chapter_idx: usize) -> FunctionName {
    orig_name.suffixed(format!("_{}", chapter_idx))
}

/// Get the `$env` that is 0-th param of async func
fn arg_ref_env() -> mir::TypedExpr {
    mir::Expr::arg_ref(0, "$env", mir::Ty::ChiikaEnv)
}

/// Get the `$cont` param of async func
/// The continuation takes an argument.
fn arg_ref_cont(arity: usize, arg_ty: mir::Ty) -> mir::TypedExpr {
    let cont_ty = mir::FunTy {
        asyncness: mir::Asyncness::Lowered,
        param_tys: vec![mir::Ty::ChiikaEnv, arg_ty],
        ret_ty: Box::new(mir::Ty::RustFuture),
    };
    mir::Expr::arg_ref(arity, "$cont", mir::Ty::Fun(cont_ty))
}

/// Get the `$async_result` which is 1-th param of chapter func
fn arg_ref_async_result(ty: mir::Ty) -> mir::TypedExpr {
    mir::Expr::arg_ref(1, "$async_result", ty)
}

fn call_chiika_env_push_frame(size: usize) -> mir::TypedExpr {
    let size_native = mir::Expr::raw_i64(size as i64);
    mir::Expr::fun_call(
        mir::Expr::func_ref(
            FunctionName::mangled("chiika_env_push_frame"),
            mir::FunTy {
                asyncness: mir::Asyncness::Lowered,
                param_tys: vec![mir::Ty::ChiikaEnv, mir::Ty::Int64],
                ret_ty: Box::new(mir::Ty::raw("Void")),
            },
        ),
        vec![arg_ref_env(), size_native],
    )
}

fn call_chiika_env_pop_frame(n_pop: usize, popped_value_ty: mir::Ty) -> mir::TypedExpr {
    let n_pop_native = mir::Expr::raw_i64(n_pop as i64);
    let env_pop = {
        let fun_ty = mir::FunTy {
            asyncness: mir::Asyncness::Lowered,
            param_tys: vec![mir::Ty::ChiikaEnv, mir::Ty::Int64],
            ret_ty: Box::new(mir::Ty::Any),
        };
        let fname = FunctionName::mangled("chiika_env_pop_frame");
        mir::Expr::func_ref(fname, fun_ty)
    };
    mir::Expr::cast(
        mir::CastType::Recover(popped_value_ty.clone()),
        mir::Expr::fun_call(env_pop, vec![arg_ref_env(), n_pop_native]),
    )
}

fn call_chiika_spawn(f: mir::TypedExpr) -> mir::TypedExpr {
    let null_cont_ty = mir::FunTy {
        asyncness: mir::Asyncness::Lowered,
        param_tys: vec![mir::Ty::ChiikaEnv, mir::Ty::raw("Void")],
        ret_ty: Box::new(mir::Ty::RustFuture),
    };
    let new_f_ty = mir::FunTy {
        asyncness: mir::Asyncness::Lowered,
        param_tys: vec![mir::Ty::ChiikaEnv, null_cont_ty.into()],
        ret_ty: Box::new(mir::Ty::RustFuture),
    };
    let new_f = (f.0, new_f_ty.clone().into());
    let fun_ty = mir::FunTy {
        asyncness: mir::Asyncness::Lowered,
        param_tys: vec![mir::Ty::Fun(new_f_ty)],
        ret_ty: Box::new(mir::Ty::raw("Void")),
    };
    let fname = FunctionName::mangled("chiika_spawn");
    mir::Expr::fun_call(mir::Expr::func_ref(fname, fun_ty), vec![new_f])
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
    fn current_name(&self) -> &FunctionName {
        &self.chaps.last().unwrap().name
    }

    fn add(&mut self, chap: Chapter) {
        self.chaps.push(chap);
    }

    fn add_stmt(&mut self, stmt: mir::TypedExpr) {
        self.chaps.last_mut().unwrap().add_stmt(stmt);
    }

    fn add_stmts(&mut self, stmts: Vec<mir::TypedExpr>) {
        self.chaps.last_mut().unwrap().add_stmts(stmts);
    }
}

#[derive(Debug)]
struct Chapter {
    stmts: Vec<mir::TypedExpr>,
    // The resulting type of the async function called with the last stmt
    async_result_ty: Option<mir::Ty>,
    name: FunctionName,
    params: Vec<mir::Param>,
    ret_ty: mir::Ty,
    // Only for the first chapter of each Shiika method
    sig: Option<MethodSignature>,
}

impl Chapter {
    fn new_original(f: &mir::Function) -> Chapter {
        if f.asyncness.is_async() {
            let async_result_ty = f.ret_ty.clone();
            let mut params = f.params.clone();
            params.push(mir::Param::new(
                mir::Ty::Fun(mir::FunTy {
                    asyncness: mir::Asyncness::Lowered,
                    param_tys: vec![mir::Ty::ChiikaEnv, async_result_ty],
                    ret_ty: Box::new(mir::Ty::RustFuture),
                }),
                "$cont",
            ));
            Self::new(f.name.clone(), params, mir::Ty::RustFuture, f.sig.clone())
        } else {
            Self::new(
                f.name.clone(),
                f.params.clone(),
                f.ret_ty.clone(),
                f.sig.clone(),
            )
        }
    }

    fn new_async_if_clause(name: FunctionName, suffix: &str) -> Chapter {
        let params = vec![mir::Param::new(mir::Ty::ChiikaEnv, "$env")];
        Self::new(
            name.suffixed(format!("'{}", suffix)),
            params,
            mir::Ty::RustFuture,
            None,
        )
    }

    fn new_async_end_if(name: FunctionName, suffix: &str, if_ty: mir::Ty) -> Chapter {
        let params = vec![
            mir::Param::new(mir::Ty::ChiikaEnv, "$env"),
            mir::Param::new(if_ty, "$ifResult"),
        ];
        Self::new(
            name.suffixed(format!("'{}", suffix)),
            params,
            mir::Ty::RustFuture,
            None,
        )
    }

    fn new_async_call_receiver(name: FunctionName, async_result_ty: mir::Ty) -> Chapter {
        let params = vec![
            mir::Param::new(mir::Ty::ChiikaEnv, "$env"),
            mir::Param::new(async_result_ty.clone(), "$async_result"),
        ];
        Self::new(name, params, mir::Ty::RustFuture, None)
    }

    fn new_beginwhile_clause(name: &FunctionName) -> Chapter {
        Self::new(
            name.suffixed("'w"),
            vec![env_param()],
            mir::Ty::RustFuture,
            None,
        )
    }

    fn new_whilebody_clause(name: &FunctionName) -> Chapter {
        Self::new(
            name.suffixed("'h"),
            vec![env_param()],
            mir::Ty::RustFuture,
            None,
        )
    }

    fn new_endwhile_clause(name: &FunctionName) -> Chapter {
        Self::new(
            name.suffixed("'q"),
            vec![env_param()],
            mir::Ty::RustFuture,
            None,
        )
    }

    fn new(
        name: FunctionName,
        params: Vec<mir::Param>,
        ret_ty: mir::Ty,
        sig: Option<MethodSignature>,
    ) -> Chapter {
        Chapter {
            stmts: vec![],
            async_result_ty: None,
            name,
            params,
            ret_ty,
            sig,
        }
    }

    fn add_stmt(&mut self, stmt: mir::TypedExpr) {
        self.stmts.push(stmt);
    }

    fn add_stmts(&mut self, stmts: Vec<mir::TypedExpr>) {
        self.stmts.extend(stmts);
    }
}

fn env_param() -> mir::Param {
    mir::Param::new(mir::Ty::ChiikaEnv, "$env")
}
