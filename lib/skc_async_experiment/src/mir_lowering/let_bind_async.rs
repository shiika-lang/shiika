//! Extract async calls into let bindings.
//!
//! Before: f(g(), h())  // g and h are async
//! After:
//!   let $a0 = g()
//!   let $a1 = h()
//!   f($a0, $a1)
//!
//! This simplifies async_splitter by ensuring only one async call
//! needs to be handled at a time.

use crate::gensym;
use crate::mir;

pub fn run(mir: mir::Program) -> mir::Program {
    let mut c = Compiler::new();
    let funcs = mir
        .funcs
        .into_iter()
        .map(|x| compile_func(&mut c, x))
        .collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(c: &mut Compiler, orig_func: mir::Function) -> mir::Function {
    if orig_func.is_sync() {
        return orig_func;
    }
    let new_body_stmts = c.run(orig_func.body_stmts);
    mir::Function {
        body_stmts: new_body_stmts,
        ..orig_func
    }
}

struct Compiler {
    gensym: gensym::Gensym,
}

impl Compiler {
    fn new() -> Self {
        Compiler {
            gensym: gensym::Gensym::new(gensym::PREFIX_LET_BIND_ASYNC),
        }
    }

    fn run(&mut self, body_stmts: mir::TypedExpr) -> mir::TypedExpr {
        let mut new_body_stmts = vec![];
        for e in mir::expr::into_exprs(body_stmts) {
            let s = self.compile_stmt(&mut new_body_stmts, e);
            new_body_stmts.push(s);
        }
        mir::Expr::exprs(new_body_stmts)
    }

    fn compile_stmt(
        &mut self,
        new_body_stmts: &mut Vec<mir::TypedExpr>,
        expr: mir::TypedExpr,
    ) -> mir::TypedExpr {
        if !expr.0.contains_async_call() {
            // No need of modification
            return expr;
        }
        match expr.0 {
            mir::Expr::IVarRef(obj_expr, idx, name) => {
                let new_obj = self.let_bind(new_body_stmts, *obj_expr);
                mir::Expr::ivar_ref(new_obj, idx, name, expr.1)
            }
            mir::Expr::EnvSet(idx, value_expr, name) => {
                let new_value = self.let_bind(new_body_stmts, *value_expr);
                mir::Expr::env_set(idx, new_value, name)
            }
            mir::Expr::FunCall(fexpr, args) => {
                let new_args: Vec<_> = args
                    .into_iter()
                    .map(|arg| self.let_bind_if_needed(new_body_stmts, arg))
                    .collect();
                mir::Expr::fun_call(*fexpr, new_args)
            }
            mir::Expr::VTableRef(obj_expr, idx, name) => {
                let new_obj = self.let_bind(new_body_stmts, *obj_expr);
                mir::Expr::vtable_ref(new_obj, idx, name, expr.1.as_fun_ty().clone())
            }
            mir::Expr::WTableRef(obj_expr, module, idx, name) => {
                let new_obj = self.let_bind(new_body_stmts, *obj_expr);
                mir::Expr::wtable_ref(new_obj, module, idx, name, expr.1.as_fun_ty().clone())
            }
            mir::Expr::If(cond, then_expr, else_expr) => {
                let new_cond = self.let_bind_if_needed(new_body_stmts, *cond);
                let new_then = self.run(*then_expr);
                let new_else = self.run(*else_expr);
                mir::Expr::if_(new_cond, new_then, new_else)
            }
            mir::Expr::While(cond, body) => {
                let new_cond = self.let_bind_if_needed(new_body_stmts, *cond);
                let new_body = self.run(*body);
                mir::Expr::while_(new_cond, new_body)
            }
            mir::Expr::Spawn(e) => {
                let new_e = self.run(*e);
                mir::Expr::spawn(new_e)
            }
            mir::Expr::LVarSet(name, e) => {
                let new_e = self.let_bind(new_body_stmts, *e);
                mir::Expr::lvar_set(name, new_e)
            }
            mir::Expr::IVarSet(obj_expr, idx, value_expr, name) => {
                let new_obj = self.let_bind_if_needed(new_body_stmts, *obj_expr);
                let new_value = self.let_bind_if_needed(new_body_stmts, *value_expr);
                mir::Expr::ivar_set(new_obj, idx, new_value, name)
            }
            mir::Expr::ConstSet(name, e) => {
                let new_e = self.let_bind(new_body_stmts, *e);
                mir::Expr::const_set(name, new_e)
            }
            mir::Expr::Return(Some(e)) => {
                let new_e = self.let_bind(new_body_stmts, *e);
                mir::Expr::return_(new_e)
            }
            mir::Expr::Return(None) => mir::Expr::return_cvoid(),
            mir::Expr::Exprs(exprs) => {
                let mut inner_stmts = vec![];
                for e in exprs {
                    let s = self.let_bind_if_needed(&mut inner_stmts, e);
                    inner_stmts.push(s);
                }
                mir::Expr::exprs(inner_stmts)
            }
            mir::Expr::Cast(cast_type, e) => {
                let new_e = self.let_bind(new_body_stmts, *e);
                mir::Expr::cast(cast_type, new_e)
            }
            mir::Expr::CreateNativeArray(elems) => {
                let new_elems: Vec<_> = elems
                    .into_iter()
                    .map(|elem| self.let_bind_if_needed(new_body_stmts, elem))
                    .collect();
                mir::Expr::create_native_array(new_elems)
            }
            mir::Expr::Unbox(e) => {
                let new_e = self.let_bind(new_body_stmts, *e);
                mir::Expr::unbox(new_e)
            }
            _ => unreachable!("extract_async_args: unexpected expr {:?}", expr),
        }
    }

    fn let_bind(
        &mut self,
        new_body_stmts: &mut Vec<mir::TypedExpr>,
        expr: mir::TypedExpr,
    ) -> mir::TypedExpr {
        let name = self.gensym.new_name();
        let ty = expr.1.clone();
        let compiled = self.compile_stmt(new_body_stmts, expr);
        new_body_stmts.push(mir::Expr::lvar_decl(name.clone(), compiled, false));
        mir::Expr::lvar_ref(name, ty)
    }

    fn let_bind_if_needed(
        &mut self,
        new_body_stmts: &mut Vec<mir::TypedExpr>,
        expr: mir::TypedExpr,
    ) -> mir::TypedExpr {
        if expr.0.contains_async_call() {
            self.let_bind(new_body_stmts, expr)
        } else {
            expr
        }
    }
}
