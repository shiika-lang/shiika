use crate::mir;
use anyhow::{bail, Context, Result};
use std::collections::HashSet;

/// Check type consistency of the HIR to detect bugs in the compiler.
pub fn run(mir: &mir::CompilationUnit) -> Result<()> {
    run_(mir).context("MIR verifier failed")
}
pub fn run_(mir: &mir::CompilationUnit) -> Result<()> {
    let program = &mir.program;
    let v = Verifier {};
    v.verify_externs(&program.externs)?;
    for f in &program.funcs {
        v.verify_function(f)?;
    }
    Ok(())
}

struct Verifier {}

impl Verifier {
    fn verify_externs(&self, externs: &[mir::Extern]) -> Result<()> {
        // Function names must be unique
        let mut names = HashSet::new();
        for e in externs {
            if names.contains(&e.name) {
                bail!("duplicate extern function name: {}", e.name);
            }
            names.insert(e.name.clone());
        }
        Ok(())
    }

    fn verify_function(&self, f: &mir::Function) -> Result<()> {
        for p in &f.params {
            assert_not_never(&p.ty)
                .context(format!("in parameter {:?}", p.name))
                .context(format!("in function {:?}", f.name))?;
        }

        self.verify_expr(f, &f.body_stmts)
            .context(format!("in function {:?}", f.name))?;
        Ok(())
    }

    fn verify_expr(&self, f: &mir::Function, e: &mir::TypedExpr) -> Result<()> {
        self.verify_expr_(f, e).context(format!("in expr {:?}", e))
    }
    fn verify_expr_(&self, f: &mir::Function, e: &mir::TypedExpr) -> Result<()> {
        use anyhow::bail;
        match &e.0 {
            mir::Expr::Number(_) => assert(&e, "number", &mir::Ty::raw("Int"))?,
            mir::Expr::PseudoVar(pv) => match pv {
                mir::PseudoVar::True | mir::PseudoVar::False => {
                    assert(&e, "pseudovar", &mir::Ty::raw("Bool"))?
                }
                mir::PseudoVar::Void => assert(&e, "pseudovar", &mir::Ty::raw("Void"))?,
            },
            mir::Expr::LVarRef(_) => (),
            mir::Expr::IVarRef(obj_expr, _, _) => {
                self.verify_expr(f, obj_expr)?;
            }
            mir::Expr::ArgRef(_, _) => (),
            mir::Expr::ConstRef(_) => (),
            mir::Expr::FuncRef(_) => (),
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
                    .try_for_each(|((i, p), a)| assert(&a, &format!("argument #{}", i), p))?;
            }
            mir::Expr::GetVTable(receiver_expr) => {
                self.verify_expr(f, receiver_expr)?;
            }
            mir::Expr::WTableRef(receiver_expr, _module, _idx, _debug_name) => {
                // TODO: Implement wtable verification similar to vtable
                self.verify_expr(f, receiver_expr)?;
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
            mir::Expr::Spawn(_) => todo!(),
            mir::Expr::Alloc(_, _) => (),
            mir::Expr::LVarDecl(_, v, _) => {
                self.verify_expr(f, v)?;
            }
            mir::Expr::LVarSet(_, v) => {
                self.verify_expr(f, v)?;
            }
            mir::Expr::IVarSet(obj_expr, _, v, _) => {
                self.verify_expr(f, obj_expr)?;
                self.verify_expr(f, v)?;
            }
            mir::Expr::ConstSet(_, v) => {
                self.verify_expr(f, v)?;
            }
            mir::Expr::Return(v) => {
                if let Some(val) = v {
                    self.verify_expr(f, val)?;
                    assert(&val, "return value", &f.ret_ty)?;
                } else {
                    if f.ret_ty != mir::Ty::CVoid {
                        bail!(
                            "return without value used for non-CVoid function (expected {:?})",
                            f.ret_ty
                        );
                    }
                }
                assert(&e, "return itself", &mir::Ty::raw("Never"))?;
            }
            mir::Expr::Exprs(es) => {
                self.verify_exprs(f, es)?;
            }
            mir::Expr::Cast(cast_type, val) => {
                self.verify_expr(f, val)?;
                match cast_type {
                    mir::CastType::Force(ty) => {
                        assert(&e, "result", ty)?;
                    }
                    mir::CastType::Upcast(ty) => {
                        assert(&e, "result", ty)?;
                    }
                    mir::CastType::ToAny => {
                        assert(&e, "result", &mir::Ty::Any)?;
                    }
                    mir::CastType::Recover(val_ty) => {
                        assert(&val, "castee", &mir::Ty::Any)?;
                        assert(&e, "result", val_ty)?;
                    }
                }
            }
            mir::Expr::CreateObject(_) => (),
            mir::Expr::CreateTypeObject(..) => (),
            mir::Expr::Unbox(val) => {
                assert(&val, "unboxee", &mir::Ty::raw("Int"))?;
                assert(&e, "result", &mir::Ty::Int64)?;
            }
            mir::Expr::RawI64(_) => assert(&e, "raw i64", &mir::Ty::Int64)?,
            mir::Expr::Nop => (),
            mir::Expr::StringLiteral(_) => (),
            mir::Expr::CreateNativeArray(elem_exprs) => {
                for elem in elem_exprs {
                    self.verify_expr(f, elem)?;
                }
            }
            mir::Expr::NativeArrayRef(arr_expr, _) => {
                self.verify_expr(f, arr_expr)?;
            }
            mir::Expr::EnvRef(_, _) => (),
            mir::Expr::EnvSet(_, v, _) => {
                self.verify_expr(f, v)?;
            }
            mir::Expr::WTableKey(_) => (),
            mir::Expr::WTableRow(_, _) => (),
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
    if !v.1.same(expected) {
        bail!(
            "expected {:?} for {for_}, but got {:?} which is {:?}",
            expected,
            v.0,
            v.1
        );
    }
    Ok(())
}

fn assert_not_never(ty: &mir::Ty) -> Result<()> {
    if *ty == mir::Ty::raw("Never") {
        bail!("must not be Ty::Never here");
    }
    Ok(())
}
