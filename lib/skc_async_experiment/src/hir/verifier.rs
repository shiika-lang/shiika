use crate::hir;
use crate::names::FunctionName;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;

/// Check type consistency of the HIR to detect bugs in the compiler.
pub fn run(hir: &hir::Program) -> Result<()> {
    let mut sigs: HashMap<_, _> = hir
        .funcs
        .iter()
        .map(|f| (f.name.clone(), f.fun_ty().clone()))
        .collect();
    for e in &hir.externs {
        sigs.insert(e.name.clone(), e.fun_ty.clone());
    }

    let v = Verifier { sigs };
    for f in &hir.funcs {
        v.verify_function(f)?;
    }
    Ok(())
}

struct Verifier {
    sigs: HashMap<FunctionName, hir::FunTy>,
}

impl Verifier {
    fn verify_function(&self, f: &hir::Function) -> Result<()> {
        for p in &f.params {
            assert_not_never(&p.ty)
                .context(format!("in parameter {:?}", p.name))
                .context(format!("in function {:?}", f.name))?;
        }

        self.verify_expr(f, &f.body_stmts)?;
        Ok(())
    }

    fn verify_expr(&self, f: &hir::Function, e: &hir::TypedExpr) -> Result<()> {
        self._verify_expr(f, e)
            .context(format!("in expr {:?}", e.0))
            .context(format!("in function {:?}", f.name))
            .context(format!("[BUG] Type verifier failed"))
    }

    fn _verify_expr(&self, f: &hir::Function, e: &hir::TypedExpr) -> Result<()> {
        match &e.0 {
            hir::Expr::Number(_) => assert(&e, "number", &hir::Ty::raw("Int"))?,
            hir::Expr::PseudoVar(_) => (),
            hir::Expr::LVarRef(_) => (),
            hir::Expr::ArgRef(idx, name) => {
                if *idx >= f.params.len() {
                    bail!("argument index out of range: {}", idx);
                }
                let param = &f.params[*idx];
                if param.name != *name {
                    bail!(
                        "argument name mismatch: expected {}, but got {}",
                        param.name,
                        name
                    );
                }
                assert(&e, "according to the function decalation", &param.ty)?;
            }
            hir::Expr::FuncRef(name) => {
                let ty_expected = self
                    .sigs
                    .get(name)
                    .with_context(|| format!("function {} not found", name))?;
                let ty_given = e.1.as_fun_ty();
                if !ty_expected.same(&ty_given) {
                    bail!(
                        "function reference {} has type {:?}, but declared as {:?}",
                        name,
                        ty_given,
                        ty_expected
                    );
                }
            }
            hir::Expr::FunCall(fexpr, args) => {
                for a in args {
                    assert_not_never(&a.1)?;
                    self.verify_expr(f, a)?;
                }
                self.verify_expr(f, fexpr)?;
                let hir::Ty::Fun(fun_ty) = &fexpr.1 else {
                    bail!("expected function, but got {:?}", fexpr.1);
                };
                fun_ty
                    .param_tys
                    .iter()
                    .enumerate()
                    .zip(args.iter())
                    .try_for_each(|((i, p), a)| assert(&a, &format!("argument {}", i), p))?;
            }
            hir::Expr::If(cond, then, els) => {
                self.verify_expr(f, cond)?;
                self.verify_expr(f, then)?;
                self.verify_expr(f, els)?;
            }
            hir::Expr::While(cond, body) => {
                self.verify_expr(f, cond)?;
                self.verify_expr(f, body)?;
            }
            hir::Expr::Alloc(_) => (),
            hir::Expr::Assign(_, v) => {
                self.verify_expr(f, v)?;
            }
            hir::Expr::Return(v) => {
                self.verify_expr(f, v)?;
                assert(&v, "return value", &f.ret_ty)?;
                assert(&e, "return itself", &hir::Ty::raw("Never"))?;
            }
            hir::Expr::Exprs(es) => {
                self.verify_exprs(f, es)?;
            }
            hir::Expr::Cast(cast_type, val) => {
                self.verify_expr(f, val)?;
                match cast_type {
                    hir::CastType::AnyToFun(fun_ty) => {
                        assert(&e, "cast type", &fun_ty.clone().into())?;
                        assert(&val, "castee", &hir::Ty::Any)?;
                        assert(&e, "result", &fun_ty.clone().into())?;
                    }
                    hir::CastType::AnyToInt => {
                        assert(&val, "castee", &hir::Ty::Any)?;
                        assert(&e, "result", &hir::Ty::raw("Int"))?;
                    }
                    hir::CastType::RawToAny => {
                        if !matches!(val.1, hir::Ty::Raw(_)) {
                            bail!("expected Ty::Raw");
                        }
                        assert(&e, "result", &hir::Ty::Any)?;
                    }
                    hir::CastType::FunToAny => {
                        assert_fun(&val.1)?;
                        assert(&e, "result", &hir::Ty::Any)?;
                    }
                }
            }
            hir::Expr::Unbox(val) => {
                assert(&val, "unboxee", &hir::Ty::raw("Int"))?;
                assert(&e, "result", &hir::Ty::Int64)?;
            }
            hir::Expr::RawI64(_) => assert(&e, "raw i64", &hir::Ty::Int64)?,
            hir::Expr::Nop => (),
            _ => panic!("not supported by verifier: {:?}", e.0),
        }
        Ok(())
    }

    fn verify_exprs(&self, f: &hir::Function, es: &[hir::TypedExpr]) -> Result<()> {
        for e in es {
            self.verify_expr(f, e)?;
        }
        Ok(())
    }
}

fn assert(v: &hir::TypedExpr, for_: &str, expected: &hir::Ty) -> Result<()> {
    if v.1 != *expected {
        bail!("expected {:?} for {for_}, but got {:?}", expected, v);
    }
    Ok(())
}

fn assert_fun(ty: &hir::Ty) -> Result<()> {
    if !matches!(ty, hir::Ty::Fun(_)) {
        bail!("expected Ty::Fun, but got {:?}", ty);
    }
    Ok(())
}

fn assert_not_never(ty: &hir::Ty) -> Result<()> {
    if *ty == hir::Ty::raw("Never") {
        bail!("must not be Ty::Never here");
    }
    Ok(())
}
