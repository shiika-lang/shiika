use crate::type_inference::{unify, Answer, Equation, TmpTy};
use anyhow::{Context, Result};
use shiika_core::ty;
use shiika_core::ty::{TermTy, TyParamKind};
use skc_hir::MethodSignature;

/// Infer type arguments of generic `.new`.
/// eg. In `Pair.new(3, true)` we can know the tyargs are `Int` and `Bool`.
pub fn infer_tyargs(sig: &MethodSignature, arg_types: &[TermTy]) -> Result<Vec<TermTy>> {
    let tprefs = ty::typarams_to_typaram_refs(&sig.typarams, TyParamKind::Method);
    let vars = tprefs.into_iter().enumerate().collect::<Vec<_>>();
    let mut ans = Answer::new();
    let param_types = sig.params.iter().map(|param| &param.ty);
    let equations = param_types
        .zip(arg_types.iter())
        .map(|(param_ty, arg_ty)| Equation(TmpTy::make(param_ty, &vars), TmpTy::from(arg_ty)))
        .collect::<Vec<_>>();
    let err = Equation::display_equations(&equations);
    unify(equations, &mut ans)?;
    let unknowns = vars
        .iter()
        .map(|(id, _)| TmpTy::unknown(*id))
        .collect::<Vec<_>>();
    ans.apply_to_vec(&unknowns)
        .context(format!("Where {}", err))
}
