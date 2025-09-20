use crate::mir;
use crate::names::FunctionName;
use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};

/// Check type consistency of the HIR to detect bugs in the compiler.
pub fn run(mir: &mir::CompilationUnit) -> Result<()> {
    run_(mir).context("MIR verifier failed")
}
pub fn run_(mir: &mir::CompilationUnit) -> Result<()> {
    let program = &mir.program;
    let mut sigs: HashMap<_, _> = program
        .funcs
        .iter()
        .map(|f| (f.name.clone(), f.fun_ty().clone()))
        .collect();
    for e in &program.externs {
        sigs.insert(e.name.clone(), e.fun_ty.clone());
    }

    let v = Verifier {
        sigs,
        vtables: &mir.vtables,
        imported_vtables: &mir.imported_vtables,
    };
    v.verify_externs(&program.externs)?;
    for f in &program.funcs {
        v.verify_function(f)?;
    }
    Ok(())
}

struct Verifier<'a> {
    sigs: HashMap<FunctionName, mir::FunTy>,
    vtables: &'a skc_mir::VTables,
    imported_vtables: &'a skc_mir::VTables,
}

impl<'a> Verifier<'a> {
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
                mir::PseudoVar::SelfRef => {
                    // TODO: Check this is the receiver type
                }
            },
            mir::Expr::LVarRef(_) => (),
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
            mir::Expr::VTableRef(receiver_expr, idx, debug_name) => {
                self.verify_expr(f, receiver_expr)?;

                let mir::Ty::Raw(class_name) = &receiver_expr.1 else {
                    bail!("receiver not Shiika value");
                };
                let class_fullname = shiika_core::names::ClassFullname(class_name.clone());
                let Some(vtable) = self
                    .vtables
                    .get(&class_fullname)
                    .or_else(|| self.imported_vtables.get(&class_fullname))
                else {
                    bail!("vtable of {class_fullname} not found")
                };
                if let Some(method_fullname) = vtable.to_vec().get(*idx) {
                    if method_fullname.first_name.0 != *debug_name {
                        bail!("debug_name not match");
                    }
                    if let Some(method_sig) = self.sigs.get(&method_fullname.clone().into()) {
                        let expected_ty = mir::Ty::Fun(method_sig.clone());
                        assert(
                            &e,
                            &format!("vtable_ref({}#{})", class_name, debug_name),
                            &expected_ty,
                        )?;
                    } else {
                        bail!(
                            "Method signature not found for {:?} in vtable ref verification",
                            method_fullname
                        );
                    }
                } else {
                    bail!(
                        "Method index {} out of bounds for vtable of {} (size: {})",
                        idx,
                        class_name,
                        vtable.size()
                    );
                }
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
            mir::Expr::LVarSet(_, v) => {
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
            mir::Expr::CreateTypeObject(_) => (),
            mir::Expr::Unbox(val) => {
                assert(&val, "unboxee", &mir::Ty::raw("Int"))?;
                assert(&e, "result", &mir::Ty::Int64)?;
            }
            mir::Expr::RawI64(_) => assert(&e, "raw i64", &mir::Ty::Int64)?,
            mir::Expr::Nop => (),
            mir::Expr::StringRef(_) => (),
            mir::Expr::EnvRef(_, _) => (),
            mir::Expr::EnvSet(_, v, _) => {
                self.verify_expr(f, v)?;
            }
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
