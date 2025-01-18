use crate::hir;
use crate::mir;
use crate::names::FunctionName;
use anyhow::{anyhow, Result};
use shiika_ast as ast;
use shiika_core::names::{method_firstname, method_fullname_raw};
use shiika_core::ty::{self, TermTy};
use skc_hir::{MethodParam, MethodSignature};
use std::collections::HashSet;

/// Create untyped HIR (i.e. contains Ty::Unknown) from AST
pub fn create(ast: &ast::Program) -> Result<hir::Program<()>> {
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
    let mut methods = vec![];
    for def in defs {
        match def {
            shiika_ast::Definition::ClassMethodDefinition { sig, body_exprs } => {
                methods.push(c.compile_func(sig, body_exprs)?);
            }
            _ => return Err(anyhow!("[wip] not supported yet: {:?}", def)),
        }
    }
    Ok(hir::Program {
        imports: Default::default(),
        imported_asyncs: Default::default(),
        methods,
    })
}

struct Compiler {
    func_names: HashSet<String>,
}

impl Compiler {
    fn compile_func(
        &self,
        sig: &shiika_ast::AstMethodSignature,
        body_exprs: &[shiika_ast::AstExpression],
    ) -> Result<hir::Method<()>> {
        let mut params = vec![];
        for p in &sig.params {
            params.push(hir::Param {
                name: p.name.clone(),
                ty: compile_ty(&p.typ)?,
            });
        }
        let ret_ty = match &sig.ret_typ {
            Some(t) => compile_ty(&t)?,
            None => ty::raw("Void"),
        };

        let mut lvars = HashSet::new();
        let mut body_stmts = body_exprs
            .iter()
            .map(|e| self.compile_expr(&sig, &mut lvars, &e))
            .collect::<Result<Vec<_>>>()?;
        for name in lvars {
            body_stmts.insert(0, untyped(hir::Expr::Alloc(name)));
        }
        insert_implicit_return(&mut body_stmts);

        Ok(hir::Method {
            asyncness: hir::Asyncness::Unknown,
            name: FunctionName::unmangled(sig.name.to_string()),
            params,
            ret_ty,
            body_stmts: untyped(hir::Expr::Exprs(body_stmts)),
        })
    }

    fn compile_expr(
        &self,
        sig: &shiika_ast::AstMethodSignature,
        lvars: &mut HashSet<String>,
        x: &shiika_ast::AstExpression,
    ) -> Result<hir::TypedExpr<()>> {
        let e = match &x.body {
            shiika_ast::AstExpressionBody::DecimalLiteral { value } => hir::Expr::Number(*value),
            shiika_ast::AstExpressionBody::PseudoVariable(token) => match token {
                shiika_ast::Token::KwTrue => hir::Expr::PseudoVar(mir::PseudoVar::True),
                shiika_ast::Token::KwFalse => hir::Expr::PseudoVar(mir::PseudoVar::False),
                _ => panic!("unexpected token: {:?}", token),
            },
            shiika_ast::AstExpressionBody::BareName(name) => {
                if lvars.contains(name) {
                    self.compile_var_ref(sig, lvars, name)?
                } else if let Some(idx) = sig.params.iter().position(|p| &p.name == name) {
                    hir::Expr::ArgRef(idx, name.to_string())
                } else if self.func_names.contains(name) {
                    hir::Expr::FuncRef(FunctionName::unmangled(name.to_string()))
                } else if name == "true" {
                    hir::Expr::PseudoVar(mir::PseudoVar::True)
                } else if name == "false" {
                    hir::Expr::PseudoVar(mir::PseudoVar::False)
                } else if name == "null" {
                    hir::Expr::PseudoVar(mir::PseudoVar::Void)
                } else {
                    return Err(anyhow!("unknown variable: {name}"));
                }
            }
            shiika_ast::AstExpressionBody::MethodCall(mcall) => {
                let method_name = mcall.method_name.0.to_string();
                let mut arg_hirs = vec![];
                for a in &mcall.args.unnamed {
                    arg_hirs.push(self.compile_expr(sig, lvars, a)?);
                }
                if let Some(e) = &mcall.receiver_expr {
                    let receiver = self.compile_expr(sig, lvars, e)?;
                    let name = method_firstname(method_name);
                    hir::Expr::MethodCall(Box::new(receiver), name, arg_hirs)
                } else {
                    let fname = FunctionName::unmangled(method_name.clone());
                    let fexpr = untyped(hir::Expr::FuncRef(fname));
                    hir::Expr::FunCall(Box::new(fexpr), arg_hirs)
                }
            }
            shiika_ast::AstExpressionBody::If {
                cond_expr,
                then_exprs,
                else_exprs,
            } => {
                let cond = self.compile_expr(sig, lvars, &cond_expr)?;
                let then = self.compile_exprs(sig, lvars, &then_exprs)?;
                let els = if let Some(else_) = else_exprs {
                    self.compile_exprs(sig, lvars, else_)?
                } else {
                    untyped(hir::Expr::PseudoVar(mir::PseudoVar::Void))
                };
                hir::Expr::If(Box::new(cond), Box::new(then), Box::new(els))
            }
            shiika_ast::AstExpressionBody::While {
                cond_expr,
                body_exprs,
            } => {
                let cond = self.compile_expr(sig, lvars, &cond_expr)?;
                let body = self.compile_exprs(sig, lvars, &body_exprs)?;
                hir::Expr::While(Box::new(cond), Box::new(body))
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
                    untyped(hir::Expr::PseudoVar(mir::PseudoVar::Void))
                };
                hir::Expr::Return(Box::new(e))
            }
            _ => return Err(anyhow!("[wip] not supported yet: {:?}", x)),
        };
        Ok((e, ()))
    }

    fn compile_var_ref(
        &self,
        sig: &shiika_ast::AstMethodSignature,
        lvars: &mut HashSet<String>,
        name: &str,
    ) -> Result<hir::Expr<()>> {
        let e = if lvars.contains(name) {
            hir::Expr::LVarRef(name.to_string())
        } else if let Some(idx) = sig.params.iter().position(|p| p.name == name) {
            hir::Expr::ArgRef(idx, name.to_string())
        } else if self.func_names.contains(name) {
            hir::Expr::FuncRef(FunctionName::unmangled(name.to_string()))
        } else if name == "true" {
            hir::Expr::PseudoVar(mir::PseudoVar::True)
        } else if name == "false" {
            hir::Expr::PseudoVar(mir::PseudoVar::False)
        } else if name == "null" {
            hir::Expr::PseudoVar(mir::PseudoVar::Void)
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
    ) -> Result<hir::TypedExpr<()>> {
        let mut es = vec![];
        for x in xs {
            es.push(self.compile_expr(sig, lvars, x)?);
        }
        Ok(untyped(hir::Expr::Exprs(es)))
    }
}

fn compile_ty(n: &shiika_ast::UnresolvedTypeName) -> Result<TermTy> {
    let t = if n.args.len() == 0 {
        let s = n.names.join("::");
        ty::raw(s)
    } else {
        todo!();
        //hir::Ty::Fun(compile_fun_ty(&n.args)?)
    };
    Ok(t)
}

pub fn compile_signature(
    type_name: String,
    sig: &shiika_ast::AstMethodSignature,
) -> MethodSignature {
    let ret_ty = match &sig.ret_typ {
        Some(t) => compile_ty(t).unwrap(),
        None => ty::raw("Void"),
    };
    let params = sig
        .params
        .iter()
        .map(|p| MethodParam {
            name: p.name.clone(),
            ty: compile_ty(&p.typ).unwrap(),
            has_default: false,
        })
        .collect();
    MethodSignature {
        fullname: method_fullname_raw(type_name, &sig.name.0),
        ret_ty,
        params,
        typarams: vec![],
    }
}

fn insert_implicit_return(exprs: &mut Vec<hir::TypedExpr<()>>) {
    match exprs.pop() {
        Some(last_expr) => {
            let needs_return = match &last_expr.0 {
                hir::Expr::Return(_) => false,
                _ => true,
            };
            if needs_return {
                exprs.push(untyped(hir::Expr::Return(Box::new(last_expr))));
            } else {
                exprs.push(last_expr);
            }
        }
        None => {
            // Insert `return Void` for empty method
            let void = untyped(hir::Expr::PseudoVar(mir::PseudoVar::Void));
            exprs.push(untyped(hir::Expr::Return(Box::new(void))));
        }
    }
}

fn untyped(e: hir::Expr<()>) -> hir::TypedExpr<()> {
    (e, ())
}
