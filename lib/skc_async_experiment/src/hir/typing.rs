use crate::hir;
use crate::mir;
use crate::names::FunctionName;
use anyhow::{anyhow, Result};
use shiika_ast::LocationSpan;
use shiika_core::names::ResolvedConstName;
use shiika_core::ty::{self, TermTy};
use skc_ast2hir::class_dict::{CallType, ClassDict};
use skc_hir::MethodSignature;
use std::collections::HashMap;

struct Typing<'f> {
    class_dict: &'f ClassDict<'f>,
    sigs: &'f HashMap<FunctionName, hir::FunTy>,
    current_func_name: &'f FunctionName,
    current_func_params: &'f [hir::Param],
    current_func_ret_ty: &'f TermTy,
}

/// Create typed HIR from untyped HIR.
pub fn run(hir: hir::Program<()>, class_dict: &ClassDict) -> Result<hir::Program<TermTy>> {
    let mut sigs = HashMap::new();
    for f in &hir.methods {
        sigs.insert(f.name.clone(), f.fun_ty());
    }

    let methods = hir
        .methods
        .into_iter()
        .map(|f| {
            let mut c = Typing {
                class_dict,
                sigs: &sigs,
                current_func_name: &f.name,
                current_func_params: &f.params,
                current_func_ret_ty: &f.ret_ty,
            };
            let new_body_stmts = c.compile_func(f.body_stmts)?;
            Ok(hir::Method {
                name: f.name,
                params: f.params,
                self_ty: f.self_ty,
                ret_ty: f.ret_ty,
                body_stmts: new_body_stmts,
            })
        })
        .collect::<Result<_>>()?;

    Ok(hir::Program {
        imports: hir.imports,
        imported_asyncs: hir.imported_asyncs,
        methods,
    })
}

impl<'f> Typing<'f> {
    fn compile_func(&mut self, body_stmts: hir::TypedExpr<()>) -> Result<hir::TypedExpr<TermTy>> {
        let mut lvars = HashMap::new();
        self.compile_expr(&mut lvars, body_stmts)
    }

    fn compile_expr(
        &mut self,
        lvars: &mut HashMap<String, TermTy>,
        e: hir::TypedExpr<()>,
    ) -> Result<hir::TypedExpr<TermTy>> {
        let new_e = match e.0 {
            hir::Expr::Number(n) => hir::Expr::number(n),
            hir::Expr::PseudoVar(p) => {
                if p == mir::PseudoVar::SelfRef {
                    // TODO: get the actual type of `self`
                    let ty = ty::meta("Main");
                    hir::Expr::self_ref(ty)
                } else {
                    hir::Expr::pseudo_var(p)
                }
            }
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
            hir::Expr::UnresolvedConstRef(names) => {
                // TODO: resolve const
                let ty = if names.0.first().unwrap() == "FOO" {
                    ty::raw("Int")
                } else {
                    ty::meta("Main")
                };
                let mut n = names.0.clone();
                n.insert(0, "Main".to_string());
                hir::Expr::const_ref(ResolvedConstName::new(n), ty)
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
                let arity = new_fexpr.1.fn_x_info().unwrap().len() - 1;
                if arity != arg_exprs.len() {
                    return Err(anyhow!(
                        "funcall arity mismatch (expected {}, got {})",
                        arity,
                        arg_exprs.len(),
                    ));
                }
                let new_arg_exprs = arg_exprs
                    .into_iter()
                    .map(|e| self.compile_expr(lvars, e))
                    .collect::<Result<_>>()?;
                hir::Expr::fun_call(new_fexpr, new_arg_exprs)
            }
            hir::Expr::MethodCall(recv, method_name, arg_exprs) => {
                let new_recv = self.compile_expr(lvars, *recv)?;
                let found = self.class_dict.lookup_method(
                    &new_recv.1,
                    &method_name,
                    &LocationSpan::todo(),
                )?;

                let arity = found.sig.params.len();
                if arity != arg_exprs.len() {
                    return Err(anyhow!(
                        "method call arity mismatch (expected {}, got {})",
                        arity,
                        arg_exprs.len(),
                    ));
                }
                let mut new_arg_exprs = arg_exprs
                    .into_iter()
                    .map(|e| self.compile_expr(lvars, e))
                    .collect::<Result<Vec<_>>>()?;
                let cast_recv = if found.call_type == CallType::Virtual {
                    hir::Expr::upcast(new_recv, found.sig.fullname.type_name.to_ty())
                } else {
                    new_recv
                };
                new_arg_exprs.insert(0, cast_recv);

                // TODO: method call via vtable/wtable
                hir::Expr::fun_call(method_func_ref(&found.sig), new_arg_exprs)
            }
            hir::Expr::If(cond, then, els) => {
                let new_cond = self.compile_expr(lvars, *cond)?;
                if new_cond.1 != ty::raw("Bool") {
                    return Err(anyhow!("condition should be bool but got {:?}", new_cond.1));
                }
                let new_then = self.compile_expr(lvars, *then)?;
                let new_els = self.compile_expr(lvars, *els)?;
                hir::Expr::if_(new_cond, new_then, new_els)
            }
            hir::Expr::While(cond, body) => {
                let new_cond = self.compile_expr(lvars, *cond)?;
                if new_cond.1 != ty::raw("Bool") {
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
                lvars.insert(name.clone(), ty::raw("Int"));
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
            hir::Expr::UnresolvedConstSet(names, rhs) => {
                // TODO: resolve const
                let mut n = names.0.clone();
                n.insert(0, "Main".to_string());
                let new_rhs = self.compile_expr(lvars, *rhs)?;
                hir::Expr::const_set(ResolvedConstName::new(n), new_rhs)
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
            _ => panic!("should not reach here: {:?}", e.0),
        };
        Ok(new_e)
    }
}

fn valid_return_type(expected: &TermTy, actual: &TermTy) -> bool {
    if actual == &ty::raw("Never") {
        true
    } else {
        expected == actual
    }
}

fn method_func_ref(sig: &MethodSignature) -> hir::TypedExpr<TermTy> {
    let fname = FunctionName::unmangled(&sig.fullname.full_name);
    let mut param_tys = sig.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>();
    param_tys.insert(0, sig.fullname.type_name.to_ty());
    let fun_ty = hir::FunTy {
        asyncness: hir::Asyncness::Unknown,
        param_tys,
        ret_ty: sig.ret_ty.clone(),
    };
    hir::Expr::func_ref(fname, fun_ty)
}
