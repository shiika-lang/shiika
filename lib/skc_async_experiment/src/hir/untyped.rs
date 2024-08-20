use crate::ast;
use crate::hir;
use anyhow::{anyhow, Result};
use std::collections::HashSet;

/// Create untyped HIR (i.e. contains Ty::Unknown) from AST
pub fn create(ast: &ast::Program) -> Result<hir::Program> {
    let func_names = ast
        .iter()
        .map(|decl| match decl {
            ast::Declaration::Extern(e) => e.name.clone(),
            ast::Declaration::Function(f) => f.name.clone(),
        })
        .collect::<HashSet<_>>();

    let c = Compiler { func_names };
    let mut externs = vec![];
    let mut funcs = vec![];
    for decl in ast {
        match decl {
            ast::Declaration::Extern(e) => externs.push(compile_extern(e)?),
            ast::Declaration::Function(f) => {
                funcs.push(c.compile_func(f)?);
            }
        }
    }
    Ok(hir::Program { externs, funcs })
}

struct Compiler {
    func_names: HashSet<String>,
}

impl Compiler {
    fn compile_func(&self, f: &ast::Function) -> Result<hir::Function> {
        let mut params = vec![];
        for p in &f.params {
            params.push(hir::Param {
                name: p.name.clone(),
                ty: compile_ty(&p.ty)?,
            });
        }
        let mut lvars = HashSet::new();
        Ok(hir::Function {
            generated: false,
            asyncness: hir::Asyncness::Unknown,
            name: f.name.clone(),
            params,
            ret_ty: compile_ty(&f.ret_ty)?,
            body_stmts: f
                .body_stmts
                .iter()
                .map(|e| self.compile_expr(&f, &mut lvars, &e))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    fn compile_expr(
        &self,
        f: &ast::Function,
        lvars: &mut HashSet<String>,
        x: &ast::Expr,
    ) -> Result<hir::TypedExpr> {
        let e = match x {
            ast::Expr::Number(i) => hir::Expr::Number(*i),
            ast::Expr::VarRef(name) => self.compile_var_ref(f, lvars, name)?,
            ast::Expr::OpCall(op, lhs, rhs) => {
                let lhs = self.compile_expr(f, lvars, lhs)?;
                let rhs = self.compile_expr(f, lvars, rhs)?;
                hir::Expr::OpCall(op.clone(), Box::new(lhs), Box::new(rhs))
            }
            ast::Expr::FunCall(fexpr, args) => {
                let fexpr = self.compile_expr(f, lvars, fexpr)?;
                let mut arg_hirs = vec![];
                for a in args {
                    arg_hirs.push(self.compile_expr(f, lvars, a)?);
                }
                hir::Expr::FunCall(Box::new(fexpr), arg_hirs)
            }
            ast::Expr::If(cond, then, els) => {
                let cond = self.compile_expr(f, lvars, &cond)?;
                let mut then = self.compile_exprs(f, lvars, &then)?;
                let mut els = if let Some(els) = &els {
                    self.compile_exprs(f, lvars, &els)?
                } else {
                    vec![]
                };
                if (ends_with_yield(&then) && ends_with_yield(&els))
                    || (ends_with_return(&then) && ends_with_return(&els))
                    || (ends_with_yield(&then) && ends_with_return(&els))
                    || (ends_with_return(&then) && ends_with_yield(&els))
                {
                    hir::Expr::If(Box::new(cond), then, els)
                } else if ends_with_yield(&then) || ends_with_yield(&els) {
                    return Err(anyhow!("yield must be in both (or neither) branches"));
                } else {
                    if !ends_with_yield(&then) && !ends_with_return(&then) {
                        then.push(hir::Expr::yield_null());
                    }
                    if !ends_with_yield(&els) && !ends_with_return(&els) {
                        els.push(hir::Expr::yield_null());
                    }
                    hir::Expr::If(Box::new(cond), then, els)
                }
            }
            ast::Expr::Yield(v) => {
                let e = self.compile_expr(f, lvars, v)?;
                hir::Expr::Yield(Box::new(e))
            }
            ast::Expr::While(cond, body) => {
                let cond = self.compile_expr(f, lvars, &cond)?;
                let body = self.compile_exprs(f, lvars, &body)?;
                hir::Expr::While(Box::new(cond), body)
            }
            ast::Expr::Spawn(func) => {
                let func = self.compile_expr(f, lvars, func)?;
                hir::Expr::Spawn(Box::new(func))
            }
            ast::Expr::Alloc(name) => {
                lvars.insert(name.clone());
                hir::Expr::Alloc(name.clone())
            }
            ast::Expr::Assign(name, rhs) => {
                let rhs = self.compile_expr(f, lvars, &rhs)?;
                hir::Expr::Assign(name.clone(), Box::new(rhs))
            }
            ast::Expr::Return(v) => {
                let e = self.compile_expr(f, lvars, v)?;
                hir::Expr::Return(Box::new(e))
            }
        };
        Ok((e, hir::Ty::Unknown))
    }

    fn compile_var_ref(
        &self,
        f: &ast::Function,
        lvars: &mut HashSet<String>,
        name: &str,
    ) -> Result<hir::Expr> {
        let e = if lvars.contains(name) {
            hir::Expr::LVarRef(name.to_string())
        } else if let Some(idx) = f.params.iter().position(|p| p.name == name) {
            hir::Expr::ArgRef(idx)
        } else if self.func_names.contains(name) {
            hir::Expr::FuncRef(name.to_string())
        } else if name == "true" {
            hir::Expr::PseudoVar(hir::PseudoVar::True)
        } else if name == "false" {
            hir::Expr::PseudoVar(hir::PseudoVar::False)
        } else if name == "null" {
            hir::Expr::PseudoVar(hir::PseudoVar::Null)
        } else {
            return Err(anyhow!("unknown variable: {name}"));
        };
        Ok(e)
    }

    fn compile_exprs(
        &self,
        f: &ast::Function,
        lvars: &mut HashSet<String>,
        xs: &[ast::Expr],
    ) -> Result<Vec<hir::TypedExpr>> {
        let mut es = vec![];
        for x in xs {
            es.push(self.compile_expr(f, lvars, x)?);
        }
        Ok(es)
    }
}

fn compile_extern(e: &ast::Extern) -> Result<hir::Extern> {
    let mut params = vec![];
    for p in &e.params {
        params.push(hir::Param {
            name: p.name.clone(),
            ty: compile_ty(&p.ty)?,
        });
    }
    Ok(hir::Extern {
        is_async: e.is_async,
        is_internal: e.is_internal,
        name: e.name.clone(),
        params,
        ret_ty: compile_ty(&e.ret_ty)?,
    })
}

fn compile_ty(x: &ast::Ty) -> Result<hir::Ty> {
    let t = match x {
        ast::Ty::Raw(s) => match &s[..] {
            "Null" => hir::Ty::Null,
            "Int" => hir::Ty::Int,
            "Bool" => hir::Ty::Bool,
            // Internally used types (in src/prelude.rs)
            "ANY" => hir::Ty::Any,
            "ENV" => hir::Ty::ChiikaEnv,
            "FUTURE" => hir::Ty::RustFuture,
            _ => return Err(anyhow!("unknown type: {s}")),
        },
        ast::Ty::Fun(f) => hir::Ty::Fun(compile_fun_ty(f)?),
    };
    Ok(t)
}

fn compile_fun_ty(x: &ast::FunTy) -> Result<hir::FunTy> {
    let mut param_tys = vec![];
    for p in &x.param_tys {
        param_tys.push(compile_ty(p)?);
    }
    let ret_ty = Box::new(compile_ty(&x.ret_ty)?);
    Ok(hir::FunTy {
        asyncness: hir::Asyncness::Unknown,
        param_tys,
        ret_ty,
    })
}

fn ends_with_yield(stmts: &[hir::TypedExpr]) -> bool {
    matches!(stmts.last(), Some((hir::Expr::Yield(_), _)))
}

fn ends_with_return(stmts: &[hir::TypedExpr]) -> bool {
    matches!(stmts.last(), Some((hir::Expr::Return(_), _)))
}
