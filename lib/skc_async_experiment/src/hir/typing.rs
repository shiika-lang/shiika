use crate::hir;
use crate::mir;
use crate::names::FunctionName;
use anyhow::{anyhow, Result};
use shiika_ast::LocationSpan;
use shiika_core::names::ConstFullname;
use shiika_core::ty::{self, TermTy};
use skc_ast2hir::class_dict::{CallType, ClassDict};
use skc_hir::MethodSignature;
use std::collections::HashMap;

/// Create typed HIR from untyped HIR.
pub fn run(
    hir: hir::Program<()>,
    class_dict: &ClassDict,
    imported_constants: &HashMap<ConstFullname, TermTy>,
) -> Result<hir::Program<TermTy>> {
    let mut sigs = HashMap::new();
    for f in &hir.methods {
        sigs.insert(f.name.clone(), f.fun_ty());
    }

    let mut new_constants = vec![];
    let mut known_consts = imported_constants.clone();
    for (name, rhs) in hir.constants {
        let mut c = Typing {
            class_dict,
            sigs: &mut sigs,
            known_consts: &known_consts,
            current_func: None,
        };
        let new_rhs = c.compile_expr(&mut HashMap::new(), rhs)?;
        known_consts.insert(name.clone(), new_rhs.1.clone());
        new_constants.push((name, new_rhs));
    }

    let methods = hir
        .methods
        .into_iter()
        .map(|mut f| {
            // Extract body_stmts
            let mut body_stmts = (hir::Expr::Number(0), ());
            std::mem::swap(&mut body_stmts, &mut f.body_stmts);

            let mut c = Typing {
                class_dict,
                sigs: &mut sigs,
                known_consts: &known_consts,
                current_func: Some(&f),
            };
            let new_body_stmts = c.compile_func(body_stmts)?;
            Ok(hir::Method {
                name: f.name,
                params: f.params,
                self_ty: f.self_ty,
                ret_ty: f.ret_ty,
                body_stmts: new_body_stmts,
            })
        })
        .collect::<Result<_>>()?;

    let new_top_exprs = hir
        .top_exprs
        .into_iter()
        .map(|e| {
            let mut c = Typing {
                class_dict,
                sigs: &mut sigs,
                known_consts: &known_consts,
                current_func: None,
            };
            c.compile_expr(&mut HashMap::new(), e)
        })
        .collect::<Result<_>>()?;

    Ok(hir::Program {
        methods,
        top_exprs: new_top_exprs,
        constants: new_constants,
    })
}

struct Typing<'f> {
    class_dict: &'f ClassDict<'f>,
    sigs: &'f mut HashMap<FunctionName, hir::FunTy>,
    known_consts: &'f HashMap<ConstFullname, TermTy>,
    current_func: Option<&'f hir::Method<()>>,
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
                    let ty = match &self.current_func {
                        Some(f) => f.self_ty.clone(),
                        _ => ty::raw("Object"),
                    };
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
                let current_func_params = &self.current_func.as_ref().unwrap().params;
                let ty = current_func_params[i].ty.clone();
                hir::Expr::arg_ref(i, s, ty)
            }
            hir::Expr::ConstRef(names) => {
                let ty = self.known_consts.get(&names).unwrap_or_else(|| {
                    panic!(
                        "unknown constant: {:?} (known_consts: {:?})",
                        names, self.known_consts
                    )
                });
                hir::Expr::const_ref(names, ty.clone())
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
            hir::Expr::LVarDecl(name, rhs) => {
                let new_rhs = self.compile_expr(lvars, *rhs)?;
                let ty = new_rhs.1.clone();
                lvars.insert(name.clone(), ty);
                hir::Expr::lvar_decl(name, new_rhs)
            }
            hir::Expr::Assign(name, val) => {
                let new_val = self.compile_expr(lvars, *val)?;
                if let Some(ty) = lvars.get(&name) {
                    if ty != &new_val.1 {
                        return Err(anyhow!(
                            "assigning type mismatch: variable `{name}' is {:?} but the value is {:?}",
                            ty,
                            new_val.1
                        ));
                    }
                } else {
                    panic!("unknown variable `{name}'");
                }
                hir::Expr::assign(name, new_val)
            }
            hir::Expr::ConstSet(names, rhs) => {
                let new_rhs = self.compile_expr(lvars, *rhs)?;
                hir::Expr::const_set(names, new_rhs)
            }
            hir::Expr::Return(val) => {
                let new_val = self.compile_expr(lvars, *val)?;
                match &self.current_func {
                    Some(f) => {
                        if !valid_return_type(&f.ret_ty, &new_val.1) {
                            return Err(anyhow!(
                                "return type mismatch: {} should return {:?} but got {:?}",
                                &f.name,
                                &f.ret_ty,
                                new_val.1
                            ));
                        }
                    }
                    None => {
                        return Err(anyhow!("return outside of method"));
                    }
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
            hir::Expr::Upcast(_, _) => unreachable!(),
            hir::Expr::CreateObject(class_name) => hir::Expr::create_object(class_name),
            hir::Expr::CreateTypeObject(type_name) => hir::Expr::create_type_object(type_name),
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
