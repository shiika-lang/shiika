use crate::hir;
use anyhow::{anyhow, Result};
use shiika_ast as ast;
use std::collections::HashSet;

/// Create untyped HIR (i.e. contains Ty::Unknown) from AST
pub fn create(ast: &ast::Program) -> Result<hir::Program> {
    let Some(topitem) = ast.toplevel_items.first() else {
        return Err(anyhow!("[wip] no top-level item"));
    };
    let shiika_ast::TopLevelItem::Def(shiika_ast::Definition::ClassDefinition { defs, .. }) =
        topitem
    else {
        return Err(anyhow!("[wip] top-level item must be a class definition"));
    };

    let mut func_names = HashSet::new();
    for def in defs {
        match def {
            shiika_ast::Definition::ClassMethodDefinition { sig, .. } => {
                func_names.insert(sig.name.0.to_string());
            }
            shiika_ast::Definition::MethodRequirementDefinition { sig } => {
                if let Some(name) = sig.name.0.strip_prefix("__async__") {
                    func_names.insert(name.to_string());
                } else if let Some(name) = sig.name.0.strip_prefix("__internal__") {
                    func_names.insert(name.to_string());
                } else {
                    func_names.insert(sig.name.0.to_string());
                }
            }
            _ => return Err(anyhow!("[wip] not supported yet: {:?}", def)),
        }
    }

    let c = Compiler { func_names };
    let mut externs = vec![];
    let mut funcs = vec![];
    for def in defs {
        match def {
            shiika_ast::Definition::ClassMethodDefinition { sig, body_exprs } => {
                funcs.push(c.compile_func(sig, body_exprs)?);
            }
            shiika_ast::Definition::MethodRequirementDefinition { sig } => {
                let (name, is_async, is_internal) =
                    if let Some(name) = sig.name.0.strip_prefix("__async__") {
                        (name.to_string(), true, false)
                    } else if let Some(name) = sig.name.0.strip_prefix("__internal__") {
                        (name.to_string(), false, true)
                    } else {
                        (sig.name.0.to_string(), false, false)
                    };
                externs.push(compile_extern(&name, sig, is_async, is_internal)?);
            }
            _ => return Err(anyhow!("[wip] not supported yet: {:?}", def)),
        }
    }
    Ok(hir::Program { externs, funcs })
}

struct Compiler {
    func_names: HashSet<String>,
}

impl Compiler {
    fn compile_func(
        &self,
        sig: &shiika_ast::AstMethodSignature,
        body_exprs: &[shiika_ast::AstExpression],
    ) -> Result<hir::Function> {
        let mut params = vec![];
        for p in &sig.params {
            params.push(hir::Param {
                name: p.name.clone(),
                ty: compile_ty(&p.typ)?,
            });
        }
        let ret_ty = match &sig.ret_typ {
            Some(t) => compile_ty(&t)?,
            None => hir::Ty::Null,
        };
        let mut lvars = HashSet::new();
        let body_stmts = body_exprs
            .iter()
            .map(|e| self.compile_expr(&sig, &mut lvars, &e))
            .collect::<Result<Vec<_>>>()?;
        let allocs = lvars
            .into_iter()
            .map(|name| (hir::Expr::Alloc(name), hir::Ty::Unknown))
            .collect::<Vec<_>>();
        Ok(hir::Function {
            generated: false,
            asyncness: hir::Asyncness::Unknown,
            name: sig.name.to_string(),
            params,
            ret_ty,
            body_stmts: allocs.into_iter().chain(body_stmts).collect(),
        })
    }

    fn compile_expr(
        &self,
        sig: &shiika_ast::AstMethodSignature,
        lvars: &mut HashSet<String>,
        x: &shiika_ast::AstExpression,
    ) -> Result<hir::TypedExpr> {
        let e = match &x.body {
            shiika_ast::AstExpressionBody::DecimalLiteral { value } => hir::Expr::Number(*value),
            shiika_ast::AstExpressionBody::BareName(name) => {
                if lvars.contains(name) {
                    self.compile_var_ref(sig, lvars, name)?
                } else if let Some(idx) = sig.params.iter().position(|p| &p.name == name) {
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
                }
            }
            shiika_ast::AstExpressionBody::MethodCall(mcall) => {
                let method_name = mcall.method_name.0.to_string();
                match &method_name[..] {
                    "+" | "-" | "*" | "/" | "<" | "<=" | ">" | ">=" | "==" | "!=" => {
                        let lhs =
                            self.compile_expr(sig, lvars, &mcall.receiver_expr.as_ref().unwrap())?;
                        let rhs =
                            self.compile_expr(sig, lvars, mcall.args.unnamed.first().unwrap())?;
                        hir::Expr::OpCall(method_name, Box::new(lhs), Box::new(rhs))
                    }
                    _ => {
                        if mcall.receiver_expr.is_some() {
                            return Err(anyhow!("[wip] receiver_expr must be None now"));
                        }
                        let fexpr = (hir::Expr::FuncRef(method_name), hir::Ty::Unknown);
                        let mut arg_hirs = vec![];
                        for a in &mcall.args.unnamed {
                            arg_hirs.push(self.compile_expr(sig, lvars, a)?);
                        }
                        hir::Expr::FunCall(Box::new(fexpr), arg_hirs)
                    }
                }
            }
            shiika_ast::AstExpressionBody::If {
                cond_expr,
                then_exprs,
                else_exprs,
            } => {
                let cond = self.compile_expr(sig, lvars, &cond_expr)?;
                let mut then = self.compile_exprs(sig, lvars, &then_exprs)?;
                let mut els = if let Some(els) = &else_exprs {
                    self.compile_exprs(sig, lvars, &els)?
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
            //shiika_ast::AstExpressionBody::Yield(v) => {
            //    let e = self.compile_expr(sig, lvars, v)?;
            //    hir::Expr::Yield(Box::new(e))
            //}
            shiika_ast::AstExpressionBody::While {
                cond_expr,
                body_exprs,
            } => {
                let cond = self.compile_expr(sig, lvars, &cond_expr)?;
                let body = self.compile_exprs(sig, lvars, &body_exprs)?;
                hir::Expr::While(Box::new(cond), body)
            }
            //shiika_ast::AstExpressionBody::Spawn(func) => {
            //    let func = self.compile_expr(sig, lvars, func)?;
            //    hir::Expr::Spawn(Box::new(func))
            //}
            shiika_ast::AstExpressionBody::LVarDecl { name, rhs, .. } => {
                lvars.insert(name.clone());
                let rhs = self.compile_expr(sig, lvars, &rhs)?;
                hir::Expr::Assign(name.clone(), Box::new(rhs))
            }
            shiika_ast::AstExpressionBody::LVarAssign { name, rhs } => {
                if !lvars.contains(name) {
                    return Err(anyhow!("unknown variable: {name}"));
                }
                let rhs = self.compile_expr(sig, lvars, &rhs)?;
                hir::Expr::Assign(name.clone(), Box::new(rhs))
            }
            shiika_ast::AstExpressionBody::Return { arg } => {
                let e = if let Some(v) = arg {
                    self.compile_expr(sig, lvars, v)?
                } else {
                    hir::Expr::pseudo_var(hir::PseudoVar::Null)
                };
                hir::Expr::Return(Box::new(e))
            }
            _ => return Err(anyhow!("[wip] not supported yet: {:?}", x)),
        };
        Ok((e, hir::Ty::Unknown))
    }

    fn compile_var_ref(
        &self,
        sig: &shiika_ast::AstMethodSignature,
        lvars: &mut HashSet<String>,
        name: &str,
    ) -> Result<hir::Expr> {
        let e = if lvars.contains(name) {
            hir::Expr::LVarRef(name.to_string())
        } else if let Some(idx) = sig.params.iter().position(|p| p.name == name) {
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
        sig: &shiika_ast::AstMethodSignature,
        lvars: &mut HashSet<String>,
        xs: &[shiika_ast::AstExpression],
    ) -> Result<Vec<hir::TypedExpr>> {
        let mut es = vec![];
        for x in xs {
            es.push(self.compile_expr(sig, lvars, x)?);
        }
        Ok(es)
    }
}

fn compile_extern(
    name: &str,
    sig: &shiika_ast::AstMethodSignature,
    is_async: bool,
    is_internal: bool,
) -> Result<hir::Extern> {
    let mut params = vec![];
    for p in &sig.params {
        params.push(hir::Param {
            name: p.name.clone(),
            ty: compile_ty(&p.typ)?,
        });
    }
    let ret_ty = match &sig.ret_typ {
        Some(t) => compile_ty(&t)?,
        None => hir::Ty::Null,
    };
    Ok(hir::Extern {
        is_async,
        is_internal,
        name: name.to_string(),
        params,
        ret_ty,
    })
}

fn compile_ty(n: &shiika_ast::UnresolvedTypeName) -> Result<hir::Ty> {
    let t = if n.args.len() == 0 {
        let s = n.names.first().unwrap();
        match &s[..] {
            "Null" => hir::Ty::Null,
            "Int" => hir::Ty::Int,
            "Bool" => hir::Ty::Bool,
            // Internally used types (in src/prelude.rs)
            "ANY" => hir::Ty::Any,
            "ENV" => hir::Ty::ChiikaEnv,
            "FUTURE" => hir::Ty::RustFuture,
            _ => return Err(anyhow!("unknown type: {s}")),
        }
    } else {
        hir::Ty::Fun(compile_fun_ty(&n.args)?)
    };
    Ok(t)
}

fn compile_fun_ty(x: &[shiika_ast::UnresolvedTypeName]) -> Result<hir::FunTy> {
    let mut param_tys = vec![];
    for p in x {
        param_tys.push(compile_ty(p)?);
    }
    let ret_ty = Box::new(param_tys.pop().unwrap());
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
