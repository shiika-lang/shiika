mod tmp_ty;
use crate::convert_exprs::block::BlockTaker;
use crate::error::type_error;
use anyhow::{anyhow, Context, Result};
use shiika_core::ty::{self, LitTy, TermTy, TyParam, TyParamKind, TyParamRef};
use std::collections::HashMap;
use std::fmt;
pub use tmp_ty::TmpTy;

type Id = usize;

#[derive(Debug)]
pub struct Infer {
    pub method_tyargs: Vec<TmpTy>,
    pub param_tys: Vec<TmpTy>,
    pub ret_ty: TmpTy,
    pub ans: Answer,
    // For error message
    vars: Vec<(Id, TyParamRef)>,
}

impl Infer {
    pub fn new(
        block_taker: &BlockTaker,
        class_typarams: &[TyParam],
        class_tyargs: &[TermTy],
        given_method_tyargs: Option<Vec<TermTy>>,
    ) -> Infer {
        let mut ans = Answer::new();
        let mut i = 0;
        let mut vars = vec![];
        let mut method_tyargs = vec![];
        // Class-wise typarams
        if !class_tyargs.is_empty() {
            for tpref in
                ty::typarams_to_typaram_refs(class_typarams, TyParamKind::Class, false).into_iter()
            {
                vars.push((i, tpref));
                ans.merge(i, TmpTy::make(&class_tyargs[i], Default::default()));
                i += 1;
            }
        }
        let n_class_typarams = i;
        // Method-wise typarams
        for tpref in
            ty::typarams_to_typaram_refs(&block_taker.typarams(), TyParamKind::Method, false)
                .into_iter()
        {
            vars.push((i, tpref));
            method_tyargs.push(TmpTy::unknown(i));
            if let Some(v) = given_method_tyargs.as_ref() {
                ans.merge(i, TmpTy::make(&v[i - n_class_typarams], Default::default()));
            }
            i += 1;
        }

        // Note: passing `vars` to TmpTy::make because these may refer
        // class-wise or method-wise type parameters.
        let param_tys = block_taker
            .param_tys()
            .iter()
            .map(|t| TmpTy::make(&t, &vars))
            .collect::<Vec<_>>();
        let ret_ty = TmpTy::make(&block_taker.ret_ty(), &vars);

        Infer {
            method_tyargs,
            param_tys,
            ret_ty,
            ans,
            vars,
        }
    }

    // Give the argument types for hint.
    // Length of arg_tys may be shorter than param_tys (when block is given).
    pub fn set_arg_tys(&mut self, arg_tys: &[&TermTy]) -> Result<()> {
        let equations = self
            .param_tys
            .iter()
            .zip(arg_tys.iter())
            .map(|(l, r)| Equation(l.clone(), TmpTy::from(r)))
            .collect::<Vec<_>>();
        unify(equations, &mut self.ans).with_context(|| self.error())
    }

    pub fn set_block_ty(&mut self, block_ty: &TermTy) -> Result<()> {
        let l = self.param_tys.last().unwrap().clone();
        let equation = Equation(l, TmpTy::from(block_ty));
        unify(vec![equation], &mut self.ans).with_context(|| self.error())
    }

    pub fn block_param_tys(&self) -> Result<Vec<TermTy>> {
        let Some(block_ty) = self.param_tys.last() else {
            return Err(anyhow!("this method does not take a block"));
        };
        let tyargs = block_ty.type_args().unwrap();
        self.ans
            .apply_to_vec(&tyargs[0..tyargs.len() - 1])
            .with_context(|| self.error())
    }

    pub fn method_tyargs(&self) -> Result<Vec<TermTy>> {
        self.ans
            .apply_to_vec(&self.method_tyargs)
            .with_context(|| self.error())
    }

    pub fn param_tys(&self) -> Result<Vec<TermTy>> {
        self.ans
            .apply_to_vec(&self.param_tys)
            .with_context(|| self.error())
    }

    pub fn ret_ty(&self) -> Result<TermTy> {
        self.ans
            .apply_to(&self.ret_ty)
            .with_context(|| self.error())
    }

    fn error(&self) -> String {
        let vs = self
            .vars
            .iter()
            .map(|(i, tpref)| {
                let wise = match tpref.kind {
                    TyParamKind::Class => "class-wise",
                    TyParamKind::Method => "method-wise",
                };
                format!("'{} be {} type parameter {}", i, wise, tpref.name)
            })
            .collect::<Vec<_>>();
        format!("Let {}", vs.join(", "))
    }
}

#[derive(Debug)]
struct Equation(TmpTy, TmpTy);

impl Equation {
    //    pub fn display_equations(equations: &[Equation]) -> String {
    //        equations
    //            .iter()
    //            .map(|e| e.display())
    //            .collect::<Vec<_>>()
    //            .join(", ")
    //    }
    //
    //    pub fn display(&self) -> String {
    //        format!("{} = {}", &self.0, &self.1)
    //    }

    /// Returns swapped version of self
    fn swap(self) -> Equation {
        Equation(self.1, self.0)
    }

    /// Resolve `Unknown(id)` with `t`
    fn substitute(&self, id: &Id, t: &TmpTy) -> Equation {
        Equation(self.0.substitute(id, t), self.1.substitute(id, t))
    }
}

#[derive(Debug, Default)]
pub struct Answer(HashMap<Id, TmpTy>);

impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items = self
            .0
            .iter()
            .map(|(id, t)| format!("'{}={}", id, t))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{{{}}}", items)
    }
}

impl Answer {
    fn new() -> Answer {
        Answer(Default::default())
    }

    /// Introduce new knowledge (Unknown(id) = t) to `self`.
    fn merge(&mut self, id: Id, t: TmpTy) {
        let mut h = self
            .0
            .drain()
            .map(|(id2, t2)| (id2, t2.substitute(&id, &t)))
            .collect::<HashMap<_, _>>();
        h.insert(id, t);
        self.0 = h
    }

    /// Apply `self` to TmpTy's
    fn apply_to_vec(&self, tmp_tys: &[TmpTy]) -> Result<Vec<TermTy>> {
        let dump = tmp_tys
            .iter()
            .map(|t| format!("{}", t))
            .collect::<Vec<_>>()
            .join(", ");
        tmp_tys
            .iter()
            .map(|tt| self.apply_to(tt))
            .collect::<Result<Vec<_>>>()
            .context(format!("On solving {}", dump))
    }

    /// Creates a `TermTy` by applying `self` to the `Unknown`s in `t`.
    /// Returns `Err` if could not remove all `Unknown`s.
    fn apply_to(&self, t: &TmpTy) -> Result<TermTy> {
        self._apply_to(t)
            .context(format!("t: {}, answer: {}", t, self))
    }

    fn _apply_to(&self, t: &TmpTy) -> Result<TermTy> {
        match t {
            TmpTy::Unknown(id) => {
                if self.0.contains_key(id) {
                    Answer(Default::default())._apply_to(self.0.get(id).unwrap())
                } else {
                    Err(type_error(format!(
                        "could not infer type parameter '{}",
                        id
                    )))
                }
            }
            TmpTy::Literal {
                base_name,
                type_args,
                is_meta,
            } => {
                let args = type_args
                    .iter()
                    .map(|a| self._apply_to(a))
                    .collect::<Result<Vec<_>>>()?;
                Ok(LitTy::new(base_name.clone(), args, *is_meta).into())
            }
            TmpTy::TyParamRef(r) => Ok(r.clone().into()),
        }
    }
}

/// Calculates `Answer` by unifying the equations.
fn unify(mut equations: Vec<Equation>, ans: &mut Answer) -> Result<()> {
    while let Some(eq) = equations.pop() {
        match eq {
            Equation(ty1, ty2) if ty1 == ty2 => {
                continue;
            }
            Equation(TmpTy::Unknown(id), ty2) => {
                if ty2.contains(id) {
                    return Err(type_error(format!(
                        "loop detected (id: {}, ty2: {:?})",
                        id, ty2
                    )));
                }
                equations = equations
                    .iter()
                    .map(|eq| eq.substitute(&id, &ty2))
                    .collect();
                ans.merge(id, ty2);
            }
            Equation(_, TmpTy::Unknown(_)) => {
                equations.push(eq.swap());
            }
            Equation(
                TmpTy::Literal {
                    base_name,
                    type_args,
                    is_meta,
                },
                TmpTy::Literal {
                    base_name: base_name2,
                    type_args: type_args2,
                    is_meta: is_meta2,
                },
            ) if base_name == base_name2
                && is_meta == is_meta2
                && type_args.len() == type_args2.len() =>
            {
                for (l, r) in type_args.iter().zip(type_args2.iter()) {
                    equations.push(Equation(l.clone(), r.clone()));
                }
            }
            _ => {
                // Skip this equation because it is not useful for resolving
                // `Unknown`s. (Note that this function is not used for type
                // checking at the moment)
                continue;
            }
        }
    }
    Ok(())
}
