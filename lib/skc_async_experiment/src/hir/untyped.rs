use crate::hir;
use crate::mir;
use crate::names::FunctionName;
use anyhow::{anyhow, Result};
use shiika_ast::{self, AstExpression, AstVisitor};
use shiika_core::names::{method_firstname, method_fullname_raw, ConstFullname, Namespace};
use shiika_core::ty::{self, TermTy};
use skc_hir::{MethodParam, MethodSignature};
use std::collections::HashSet;

/// Create untyped HIR (i.e. contains Ty::Unknown) from AST
pub fn create(ast: &shiika_ast::Program) -> Result<hir::Program<()>> {
    let mut v = Visitor::new();
    v.walk_program(ast)?;

    Ok(hir::Program {
        top_exprs: v.top_exprs,
        methods: v.methods,
        constants: v.constants,
    })
}

struct Visitor {
    methods: Vec<hir::Method<()>>,
    known_consts: HashSet<ConstFullname>,
    constants: Vec<(ConstFullname, hir::TypedExpr<()>)>,
    top_exprs: Vec<hir::TypedExpr<()>>,
}
impl Visitor {
    fn new() -> Self {
        Visitor {
            methods: vec![],
            known_consts: HashSet::new(),
            constants: vec![],
            top_exprs: vec![],
        }
    }
}
impl AstVisitor for Visitor {
    fn visit_method_definition(
        &mut self,
        namespace: &shiika_core::names::Namespace,
        is_instance: bool,
        _is_initializer: bool,
        sig: &shiika_ast::AstMethodSignature,
        body_exprs: &Vec<shiika_ast::AstExpression>,
    ) -> Result<()> {
        let self_ty = if is_instance {
            ty::raw(namespace.string())
        } else {
            ty::meta(namespace.string())
        };

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

        let c = Compiler::new(namespace, &self.known_consts);
        let body_stmts = c.compile_body(&sig.params, body_exprs)?;

        let m = hir::Method {
            name: FunctionName::method(&self_ty.fullname.0, &sig.name.0),
            params,
            ret_ty,
            self_ty: Some(self_ty),
            body_stmts,
        };
        self.methods.push(m);
        Ok(())
    }

    fn visit_type_definition(&mut self, namespace: &Namespace, name: &str) -> Result<()> {
        self.known_consts.insert(namespace.const_fullname(name));
        Ok(())
    }

    fn visit_const_definition(
        &mut self,
        namespace: &shiika_core::names::Namespace,
        name: &str,
        expr: &AstExpression,
    ) -> Result<()> {
        let const_name = namespace.const_fullname(name);

        let c = Compiler::new(namespace, &self.known_consts);
        let compiled = c.compile_expr(&[], &mut HashSet::new(), expr)?;

        self.constants.push((const_name.clone(), compiled));
        self.known_consts.insert(const_name);
        Ok(())
    }

    fn visit_toplevel_expr(&mut self, expr: &shiika_ast::AstExpression) -> Result<()> {
        let top_ns = Namespace::root();
        let c = Compiler::new(&top_ns, &self.known_consts);
        let compiled = c.compile_expr(&[], &mut HashSet::new(), expr)?;
        self.top_exprs.push(compiled);
        Ok(())
    }
}

struct Compiler<'a> {
    namespace: &'a Namespace,
    consts: &'a HashSet<ConstFullname>,
}

impl<'a> Compiler<'a> {
    fn new(namespace: &'a Namespace, consts: &'a HashSet<ConstFullname>) -> Self {
        Compiler { namespace, consts }
    }

    fn compile_body(
        &self,
        params: &[shiika_ast::Param],
        body_exprs: &[shiika_ast::AstExpression],
    ) -> Result<hir::TypedExpr<()>> {
        let mut lvars = HashSet::new();
        let mut body_stmts = body_exprs
            .iter()
            .map(|e| self.compile_expr(params, &mut lvars, &e))
            .collect::<Result<Vec<_>>>()?;
        for name in lvars {
            body_stmts.insert(0, untyped(hir::Expr::Alloc(name)));
        }
        insert_implicit_return(&mut body_stmts);
        Ok(untyped(hir::Expr::Exprs(body_stmts)))
    }

    fn compile_expr(
        &self,
        params: &[shiika_ast::Param],
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
                    self.compile_var_ref(params, lvars, name)?
                } else if let Some(idx) = params.iter().position(|p| &p.name == name) {
                    hir::Expr::ArgRef(idx, name.to_string())
                } else if name == "true" {
                    hir::Expr::PseudoVar(mir::PseudoVar::True)
                } else if name == "false" {
                    hir::Expr::PseudoVar(mir::PseudoVar::False)
                } else if name == "null" {
                    hir::Expr::PseudoVar(mir::PseudoVar::Void)
                } else {
                    let receiver = untyped(hir::Expr::PseudoVar(mir::PseudoVar::SelfRef));
                    hir::Expr::MethodCall(Box::new(receiver), method_firstname(name), vec![])
                }
            }
            shiika_ast::AstExpressionBody::CapitalizedName(unresolved_const_name) => {
                let fullname = lookup_const(self.consts, &unresolved_const_name.0, &self.namespace)
                    .ok_or_else(|| anyhow!("unknown constant: {:?}", unresolved_const_name))?;
                hir::Expr::ConstRef(fullname)
            }
            shiika_ast::AstExpressionBody::MethodCall(mcall) => {
                let method_name = mcall.method_name.0.to_string();
                let mut arg_hirs = vec![];
                for a in &mcall.args.unnamed {
                    arg_hirs.push(self.compile_expr(params, lvars, a)?);
                }
                let receiver = if let Some(e) = &mcall.receiver_expr {
                    self.compile_expr(params, lvars, e)?
                } else {
                    untyped(hir::Expr::PseudoVar(mir::PseudoVar::SelfRef))
                };
                let name = method_firstname(method_name);
                hir::Expr::MethodCall(Box::new(receiver), name, arg_hirs)
            }
            shiika_ast::AstExpressionBody::If {
                cond_expr,
                then_exprs,
                else_exprs,
            } => {
                let cond = self.compile_expr(params, lvars, &cond_expr)?;
                let then = self.compile_exprs(params, lvars, &then_exprs)?;
                let els = if let Some(else_) = else_exprs {
                    self.compile_exprs(params, lvars, else_)?
                } else {
                    untyped(hir::Expr::PseudoVar(mir::PseudoVar::Void))
                };
                hir::Expr::If(Box::new(cond), Box::new(then), Box::new(els))
            }
            shiika_ast::AstExpressionBody::While {
                cond_expr,
                body_exprs,
            } => {
                let cond = self.compile_expr(params, lvars, &cond_expr)?;
                let body = self.compile_exprs(params, lvars, &body_exprs)?;
                hir::Expr::While(Box::new(cond), Box::new(body))
            }
            //shiika_ast::AstExpressionBody::Spawn(func) => {
            //    let func = self.compile_expr(sig, lvars, func)?;
            //    hir::Expr::Spawn(Box::new(func))
            //}
            shiika_ast::AstExpressionBody::LVarDecl { name, rhs, .. } => {
                lvars.insert(name.clone());
                let rhs = self.compile_expr(params, lvars, &rhs)?;
                hir::Expr::Assign(name.clone(), Box::new(rhs))
            }
            shiika_ast::AstExpressionBody::LVarAssign { name, rhs } => {
                if !lvars.contains(name) {
                    return Err(anyhow!("unknown variable: {name}"));
                }
                let rhs = self.compile_expr(params, lvars, &rhs)?;
                hir::Expr::Assign(name.clone(), Box::new(rhs))
            }
            shiika_ast::AstExpressionBody::ConstAssign { names, rhs } => {
                // Note: `names` is already resolved here
                let new_rhs = self.compile_expr(params, lvars, &rhs)?;
                hir::Expr::ConstSet(ConstFullname::new(names.clone()), Box::new(new_rhs))
            }
            shiika_ast::AstExpressionBody::Return { arg } => {
                let e = if let Some(v) = arg {
                    self.compile_expr(params, lvars, v)?
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
        params: &[shiika_ast::Param],
        lvars: &mut HashSet<String>,
        name: &str,
    ) -> Result<hir::Expr<()>> {
        let e = if lvars.contains(name) {
            hir::Expr::LVarRef(name.to_string())
        } else if let Some(idx) = params.iter().position(|p| p.name == name) {
            hir::Expr::ArgRef(idx, name.to_string())
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
        params: &[shiika_ast::Param],
        lvars: &mut HashSet<String>,
        xs: &[shiika_ast::AstExpression],
    ) -> Result<hir::TypedExpr<()>> {
        let mut es = vec![];
        for x in xs {
            es.push(self.compile_expr(params, lvars, x)?);
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

fn lookup_const(
    consts: &HashSet<ConstFullname>,
    names: &[String],
    namespace: &Namespace,
) -> Option<ConstFullname> {
    let mut ns = namespace.clone();
    loop {
        let fullname = ns.const_fullname(&names.join("::"));
        if consts.contains(&fullname) {
            return Some(fullname);
        }
        match ns.parent() {
            Some(parent) => ns = parent,
            None => return None,
        }
    }
}
