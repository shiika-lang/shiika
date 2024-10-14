use crate::hir;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

struct Typing<'f> {
    sigs: HashMap<String, hir::FunTy>,
    current_func_name: Option<&'f String>,
    current_func_params: Option<&'f [hir::Param]>,
    current_func_ret_ty: Option<&'f hir::Ty>,
}

/// Create typed HIR from untyped HIR.
pub fn run(hir: &mut hir::Program) -> Result<()> {
    let mut c = Typing {
        sigs: HashMap::new(),
        current_func_name: None,
        current_func_params: None,
        current_func_ret_ty: None,
    };
    for e in &hir.externs {
        c.sigs.insert(e.name.clone(), e.fun_ty.clone());
    }
    for f in &hir.funcs {
        c.sigs.insert(f.name.clone(), f.fun_ty());
    }

    for f in hir.funcs.iter_mut() {
        c.compile_func(f)?;
    }

    Ok(())
}

impl<'f> Typing<'f> {
    fn compile_func(&mut self, func: &'f mut hir::Function) -> Result<()> {
        self.current_func_name = Some(&func.name);
        self.current_func_params = Some(&func.params);
        self.current_func_ret_ty = Some(&func.ret_ty);
        let mut lvars = HashMap::new();
        func.body_stmts
            .iter_mut()
            .try_for_each(|e| self.compile_expr(&mut lvars, e))?;
        Ok(())
    }

    fn compile_expr(
        &mut self,
        lvars: &mut HashMap<String, hir::Ty>,
        e: &mut hir::TypedExpr,
    ) -> Result<()> {
        match &mut e.0 {
            hir::Expr::Number(_) => e.1 = hir::Ty::Int,
            hir::Expr::PseudoVar(hir::PseudoVar::True) => e.1 = hir::Ty::Bool,
            hir::Expr::PseudoVar(hir::PseudoVar::False) => e.1 = hir::Ty::Bool,
            hir::Expr::PseudoVar(hir::PseudoVar::Void) => e.1 = hir::Ty::Void,
            hir::Expr::LVarRef(name) => {
                if let Some(ty) = lvars.get(name) {
                    e.1 = ty.clone();
                } else {
                    return Err(anyhow!("[BUG] unknown variable `{name}'"));
                }
            }
            hir::Expr::ArgRef(i) => e.1 = self.current_func_params.unwrap()[*i].ty.clone(),
            hir::Expr::FuncRef(name) => {
                if let Some(fun_ty) = self.sigs.get(name) {
                    e.1 = hir::Ty::Fun(fun_ty.clone());
                } else {
                    return Err(anyhow!("[BUG] unknown function `{name}'"));
                }
            }
            hir::Expr::FunCall(fexpr, arg_exprs) => {
                self.compile_expr(lvars, &mut *fexpr)?;
                let hir::Ty::Fun(fun_ty) = &fexpr.1 else {
                    return Err(anyhow!("not a function: {:?}", fexpr));
                };
                if fun_ty.param_tys.len() != arg_exprs.len() {
                    return Err(anyhow!(
                        "funcall arity mismatch (expected {}, got {}): {:?}",
                        fun_ty.param_tys.len(),
                        arg_exprs.len(),
                        e
                    ));
                }
                for e in arg_exprs {
                    self.compile_expr(lvars, e)?;
                }
                e.1 = *fun_ty.ret_ty.clone();
            }
            hir::Expr::If(cond, then, els) => {
                self.compile_expr(lvars, &mut *cond)?;
                if cond.1 != hir::Ty::Bool {
                    return Err(anyhow!("condition should be bool but got {:?}", cond.1));
                }
                self.compile_expr(lvars, then)?;
                let t1 = &then.1;
                let t2 = if let Some(els) = els {
                    self.compile_expr(lvars, els)?;
                    &els.1
                } else {
                    &hir::Ty::Void
                };
                let t = if *t1 == hir::Ty::Void {
                    t2
                } else if *t2 == hir::Ty::Void {
                    t1
                } else if t1 != t2 {
                    return Err(anyhow!(
                        "then and else should have the same type but got {:?} and {:?}",
                        t1,
                        t2
                    ));
                } else {
                    t1
                };
                e.1 = t.clone();
            }
            //hir::Expr::While(cond, body) => {
            //    self.compile_expr(lvars, cond)?;
            //    self.compile_expr(lvars, body)?;
            //    e.1 = hir::Ty::Void;
            //}
            hir::Expr::Spawn(func) => {
                self.compile_expr(lvars, func)?;
                e.1 = hir::Ty::Void;
            }
            hir::Expr::Alloc(name) => {
                // Milika vars are always Int now
                lvars.insert(name.clone(), hir::Ty::Int);
                e.1 = hir::Ty::Void;
            }
            hir::Expr::Assign(_, val) => {
                self.compile_expr(lvars, val)?;
                e.1 = hir::Ty::Void;
            }
            hir::Expr::Return(val) => {
                self.compile_expr(lvars, val)?;
                if val.1 != *self.current_func_ret_ty.unwrap() {
                    return Err(anyhow!(
                        "return type mismatch: {} should return {:?} but got {:?}",
                        self.current_func_name.unwrap(),
                        self.current_func_ret_ty.unwrap(),
                        val.1
                    ));
                }
                e.1 = hir::Ty::Never;
            }
            hir::Expr::Exprs(exprs) => {
                for e in exprs.iter_mut() {
                    self.compile_expr(lvars, e)?;
                }
                e.1 = exprs.last().unwrap().1.clone();
            }
            hir::Expr::Cast(_, _) => {
                return Err(anyhow!("[BUG] Cast unexpected here"));
            }
            _ => panic!("must not occur in hir::typing: {:?}", e.0),
        };
        Ok(())
    }
}
