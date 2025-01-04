use crate::hir;
use crate::names::FunctionName;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

struct Typing<'f> {
    sigs: &'f HashMap<FunctionName, hir::FunTy>,
    current_func_name: &'f FunctionName,
    current_func_params: &'f [hir::Param],
    current_func_ret_ty: &'f hir::Ty,
}

/// Create typed HIR from untyped HIR.
pub fn run(hir: hir::Program<()>) -> Result<hir::Program<hir::Ty>> {
    let mut sigs = HashMap::new();
    for e in &hir.externs {
        sigs.insert(e.name.clone(), e.fun_ty.clone());
    }
    for f in &hir.methods {
        sigs.insert(f.name.clone(), f.fun_ty());
    }

    let methods = hir
        .methods
        .into_iter()
        .map(|f| {
            let mut c = Typing {
                sigs: &sigs,
                current_func_name: &f.name,
                current_func_params: &f.params,
                current_func_ret_ty: &f.ret_ty,
            };
            let new_body_stmts = c.compile_func(f.body_stmts)?;
            Ok(hir::Method {
                asyncness: f.asyncness,
                name: f.name,
                params: f.params,
                ret_ty: f.ret_ty,
                body_stmts: new_body_stmts,
            })
        })
        .collect::<Result<_>>()?;

    Ok(hir::Program {
        externs: hir.externs,
        methods,
    })
}

impl<'f> Typing<'f> {
    fn compile_func(&mut self, body_stmts: hir::TypedExpr<()>) -> Result<hir::TypedExpr<hir::Ty>> {
        let mut lvars = HashMap::new();
        self.compile_expr(&mut lvars, body_stmts)
    }

    fn compile_expr(
        &mut self,
        lvars: &mut HashMap<String, hir::Ty>,
        e: hir::TypedExpr<()>,
    ) -> Result<hir::TypedExpr<hir::Ty>> {
        let new_e = match e.0 {
            hir::Expr::Number(n) => hir::Expr::number(n),
            hir::Expr::PseudoVar(p) => hir::Expr::pseudo_var(p),
            hir::Expr::LVarRef(name) => {
                if let Some(ty) = lvars.get(&name) {
                    hir::Expr::lvar_ref(name, ty.clone())
                } else {
                    return Err(anyhow!("[BUG] unknown variable `{name}'"));
                }
            }
            hir::Expr::ArgRef(i, s) => {
                let ty = self.current_func_params[i].ty.clone();
                hir::Expr::arg_ref(i, s, ty)
            }
            hir::Expr::FuncRef(name) => {
                if let Some(fun_ty) = self.sigs.get(&name) {
                    hir::Expr::func_ref(name, fun_ty.clone())
                } else {
                    return Err(anyhow!("[BUG] unknown function `{name}'"));
                }
            }
            hir::Expr::FunCall(fexpr, arg_exprs) => {
                let new_fexpr = self.compile_expr(lvars, *fexpr)?;
                let hir::Ty::Fun(fun_ty) = &new_fexpr.1 else {
                    return Err(anyhow!("not a function"));
                };
                if fun_ty.param_tys.len() != arg_exprs.len() {
                    return Err(anyhow!(
                        "funcall arity mismatch (expected {}, got {})",
                        fun_ty.param_tys.len(),
                        arg_exprs.len(),
                    ));
                }
                let new_arg_exprs = arg_exprs
                    .into_iter()
                    .map(|e| self.compile_expr(lvars, e))
                    .collect::<Result<_>>()?;
                hir::Expr::fun_call(new_fexpr, new_arg_exprs)
            }
            hir::Expr::If(cond, then, els) => {
                let new_cond = self.compile_expr(lvars, *cond)?;
                if new_cond.1 != hir::Ty::raw("Bool") {
                    return Err(anyhow!("condition should be bool but got {:?}", new_cond.1));
                }
                let new_then = self.compile_expr(lvars, *then)?;
                let new_els = self.compile_expr(lvars, *els)?;
                hir::Expr::if_(new_cond, new_then, new_els)
            }
            hir::Expr::While(cond, body) => {
                let new_cond = self.compile_expr(lvars, *cond)?;
                if new_cond.1 != hir::Ty::raw("Bool") {
                    return Err(anyhow!("condition should be bool but got {:?}", new_cond.1));
                }
                let new_body = self.compile_expr(lvars, *body)?;
                hir::Expr::while_(new_cond, new_body)
            }
            hir::Expr::Spawn(func) => {
                let new_func = self.compile_expr(lvars, *func)?;
                hir::Expr::spawn(new_func)
            }
            hir::Expr::Alloc(name) => {
                // Milika vars are always Int now
                lvars.insert(name.clone(), hir::Ty::raw("Int"));
                hir::Expr::alloc(name)
            }
            hir::Expr::Assign(name, val) => {
                let new_val = self.compile_expr(lvars, *val)?;
                if let Some(ty) = lvars.get(&name) {
                    if ty != &new_val.1 {
                        return Err(anyhow!(
                            "type mismatch: variable `{name}' should be {:?} but got {:?}",
                            ty,
                            new_val.1
                        ));
                    }
                } else {
                    panic!("unknown variable `{name}'");
                }
                hir::Expr::assign(name, new_val)
            }
            hir::Expr::Return(val) => {
                let new_val = self.compile_expr(lvars, *val)?;
                if !valid_return_type(self.current_func_ret_ty, &new_val.1) {
                    return Err(anyhow!(
                        "return type mismatch: {} should return {:?} but got {:?}",
                        self.current_func_name,
                        self.current_func_ret_ty,
                        new_val.1
                    ));
                }
                hir::Expr::return_(new_val)
            }
            hir::Expr::Exprs(exprs) => {
                let new_exprs = exprs
                    .into_iter()
                    .map(|e| self.compile_expr(lvars, e))
                    .collect::<Result<_>>()?;
                hir::Expr::exprs(new_exprs)
            }
        };
        Ok(new_e)
    }
}

fn valid_return_type(expected: &hir::Ty, actual: &hir::Ty) -> bool {
    if actual == &hir::Ty::raw("Never") {
        true
    } else {
        expected == actual
    }
}
