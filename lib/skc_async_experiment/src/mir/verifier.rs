use crate::mir;
use crate::names::FunctionName;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;

/// Check type consistency of the HIR to detect bugs in the compiler.
pub fn run(mir: &mir::Program) -> Result<()> {
    let mut sigs: HashMap<_, _> = mir
        .funcs
        .iter()
        .map(|f| (f.name.clone(), f.fun_ty().clone()))
        .collect();
    for e in &mir.externs {
        sigs.insert(e.name.clone(), e.fun_ty.clone());
    }

    let v = Verifier { sigs };
    for f in &mir.funcs {
        v.verify_function(f)?;
    }
    Ok(())
}

struct Verifier {
    sigs: HashMap<FunctionName, mir::FunTy>,
}

impl Verifier {
    fn verify_function(&self, f: &mir::Function) -> Result<()> {
        for p in &f.params {
            assert_not_never(&p.ty)
                .context(format!("in parameter {:?}", p.name))
                .context(format!("in function {:?}", f.name))?;
        }

        self.verify_expr(f, &f.body_stmts)?;
        Ok(())
    }

    fn verify_expr(&self, f: &mir::Function, e: &mir::TypedExpr) -> Result<()> {
        self._verify_expr(f, e)
            .context(format!("in expr {:?}", e.0))
            .context(format!("in function {:?}", f.name))
            .context(format!("[BUG] Type verifier failed"))
    }

    fn _verify_expr(&self, f: &mir::Function, e: &mir::TypedExpr) -> Result<()> {
        match &e.0 {
            mir::Expr::Number(_) => assert(&e, "number", &mir::Ty::raw("Int"))?,
            mir::Expr::PseudoVar(_) => (),
            mir::Expr::LVarRef(_) => (),
            mir::Expr::ArgRef(idx, name) => {
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
            mir::Expr::ConstRef(_name) => (),
            mir::Expr::FuncRef(name) => {
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
            mir::Expr::FunCall(fexpr, args) => {
                for a in args {
                    assert_not_never(&a.1)?;
                    self.verify_expr(f, a)?;
                }
                self.verify_expr(f, fexpr)?;
                let mir::Ty::Fun(fun_ty) = &fexpr.1 else {
                    bail!("expected function, but got {:?}", fexpr.1);
                };
                fun_ty
                    .param_tys
                    .iter()
                    .enumerate()
                    .zip(args.iter())
                    .try_for_each(|((i, p), a)| assert(&a, &format!("argument {}", i), p))?;
            }
            mir::Expr::If(cond, then, els) => {
                self.verify_expr(f, cond)?;
                self.verify_expr(f, then)?;
                self.verify_expr(f, els)?;
            }
            mir::Expr::While(cond, body) => {
                self.verify_expr(f, cond)?;
                self.verify_expr(f, body)?;
            }
            mir::Expr::Alloc(_) => (),
            mir::Expr::Assign(_, v) => {
                self.verify_expr(f, v)?;
            }
            mir::Expr::ConstSet(_, v) => {
                self.verify_expr(f, v)?;
            }
            mir::Expr::Return(v) => {
                self.verify_expr(f, v)?;
                assert(&v, "return value", &f.ret_ty)?;
                assert(&e, "return itself", &mir::Ty::raw("Never"))?;
            }
            mir::Expr::Exprs(es) => {
                self.verify_exprs(f, es)?;
            }
            mir::Expr::Cast(cast_type, val) => {
                self.verify_expr(f, val)?;
                match cast_type {
                    mir::CastType::Upcast(ty) => {
                        assert(&e, "result", ty)?;
                    }
                    mir::CastType::AnyToFun(fun_ty) => {
                        assert(&e, "cast type", &fun_ty.clone().into())?;
                        assert(&val, "castee", &mir::Ty::Any)?;
                        assert(&e, "result", &fun_ty.clone().into())?;
                    }
                    mir::CastType::AnyToInt => {
                        assert(&val, "castee", &mir::Ty::Any)?;
                        assert(&e, "result", &mir::Ty::raw("Int"))?;
                    }
                    mir::CastType::RawToAny => {
                        if !matches!(val.1, mir::Ty::Raw(_)) {
                            bail!("expected Ty::Raw");
                        }
                        assert(&e, "result", &mir::Ty::Any)?;
                    }
                    mir::CastType::FunToAny => {
                        assert_fun(&val.1)?;
                        assert(&e, "result", &mir::Ty::Any)?;
                    }
                }
            }
            mir::Expr::CreateTypeObject(_) => (),
            mir::Expr::Unbox(val) => {
                assert(&val, "unboxee", &mir::Ty::raw("Int"))?;
                assert(&e, "result", &mir::Ty::Int64)?;
            }
            mir::Expr::RawI64(_) => assert(&e, "raw i64", &mir::Ty::Int64)?,
            mir::Expr::Nop => (),
            _ => panic!("not supported by verifier: {:?}", e.0),
        }
        Ok(())
    }

    fn verify_exprs(&self, f: &mir::Function, es: &[mir::TypedExpr]) -> Result<()> {
        for e in es {
            self.verify_expr(f, e)?;
        }
        Ok(())
    }
}

fn assert(v: &mir::TypedExpr, for_: &str, expected: &mir::Ty) -> Result<()> {
    if v.1 != *expected {
        bail!("expected {:?} for {for_}, but got {:?}", expected, v);
    }
    Ok(())
}

fn assert_fun(ty: &mir::Ty) -> Result<()> {
    if !matches!(ty, mir::Ty::Fun(_)) {
        bail!("expected Ty::Fun, but got {:?}", ty);
    }
    Ok(())
}

fn assert_not_never(ty: &mir::Ty) -> Result<()> {
    if *ty == mir::Ty::raw("Never") {
        bail!("must not be Ty::Never here");
    }
    Ok(())
}
