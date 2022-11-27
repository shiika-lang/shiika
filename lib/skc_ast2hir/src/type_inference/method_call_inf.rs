use crate::type_inference::{unify, Answer, Equation, TmpTy};
use anyhow::Result;
use shiika_core::ty;
use shiika_core::ty::{TermTy, TyParamKind};
use skc_hir::MethodSignature;

/// Phase 1
#[derive(Debug)]
pub struct MethodCallInf1 {
    has_block: bool,
    pub method_arg_tys: Vec<TmpTy>,
    pub method_ret_ty: TmpTy,
    pub answer: Answer,
}

impl MethodCallInf1 {
    pub fn new(sig: &MethodSignature, has_block: bool) -> MethodCallInf1 {
        let tprefs = ty::typarams_to_typaram_refs(&sig.typarams, TyParamKind::Method);
        let vars = tprefs.into_iter().enumerate().collect::<Vec<_>>();
        let method_arg_tys = sig
            .params
            .iter()
            .map(|param| TmpTy::make(&param.ty, &vars))
            .collect::<Vec<_>>();
        let method_ret_ty = TmpTy::make(&sig.ret_ty, &vars);

        MethodCallInf1 {
            has_block,
            method_arg_tys,
            method_ret_ty,
            answer: Default::default(),
        }
    }

    pub fn infer_block(sig: &MethodSignature) -> MethodCallInf1 {
        let vars = [];
        let method_arg_tys = sig
            .params
            .iter()
            .map(|param| TmpTy::make(&param.ty, &vars))
            .collect::<Vec<_>>();
        let method_ret_ty = TmpTy::make(&sig.ret_ty, &vars);

        MethodCallInf1 {
            has_block: true,
            method_arg_tys,
            method_ret_ty,
            answer: Default::default(),
        }
    }

    pub fn pre_block_arg_tys(&self) -> &[TmpTy] {
        debug_assert!(&self.has_block);
        let tys = &self.method_arg_tys;
        &tys[..tys.len() - 1]
    }

    pub fn block_param_tys(&self) -> &[TmpTy] {
        debug_assert!(&self.has_block);
        let block_ty = self.method_arg_tys.last().unwrap();
        let tys = block_ty.type_args().unwrap();
        &tys[..tys.len() - 1]
    }

    pub fn block_ret_ty(&self) -> &TmpTy {
        debug_assert!(&self.has_block);
        let block_ty = self.method_arg_tys.last().unwrap();
        let block_param_tys = block_ty.type_args().unwrap();
        block_param_tys.last().unwrap()
    }
}

/// Phase 2
/// Block parameter types are solved.
/// Only used when block exists.
#[derive(Debug)]
pub struct MethodCallInf2 {
    pub solved_pre_block_arg_tys: Vec<TermTy>,
    pub block_ret_ty: TmpTy,
    pub method_ret_ty: TmpTy,
    pub solved_block_param_tys: Vec<TermTy>,
    pub answer: Answer,
}

impl MethodCallInf2 {
    fn new(
        inf: MethodCallInf1,
        solved_pre_block_arg_tys: Vec<TermTy>,
        solved_block_param_tys: Vec<TermTy>,
    ) -> MethodCallInf2 {
        debug_assert!(&inf.has_block);
        MethodCallInf2 {
            solved_pre_block_arg_tys,
            block_ret_ty: inf.block_ret_ty().clone(),
            method_ret_ty: inf.method_ret_ty,
            solved_block_param_tys,
            answer: inf.answer,
        }
    }
}

/// Phase 3 (All solved)
#[derive(Debug)]
pub struct MethodCallInf3 {
    pub solved_method_arg_tys: Vec<TermTy>,
    // Not used in current implementation
    //solved_method_ret_ty: TermTy,
}

impl MethodCallInf3 {
    fn with_block(inf: MethodCallInf2, solved_block_ret_ty: TermTy) -> MethodCallInf3 {
        let solved_block_ty = ty::fn_ty(inf.solved_block_param_tys, solved_block_ret_ty);
        let mut solved_method_arg_tys = inf.solved_pre_block_arg_tys;
        solved_method_arg_tys.push(solved_block_ty);
        MethodCallInf3 {
            solved_method_arg_tys,
        }
    }
}

pub fn infer_block_param(
    mut inf: MethodCallInf1,
    pre_block_arg_tys: &[&TermTy],
) -> Result<MethodCallInf2> {
    let equations = inf
        .method_arg_tys
        .iter()
        .zip(pre_block_arg_tys.iter())
        .map(|(l, r)| Equation(l.clone(), TmpTy::from(r)))
        .collect::<Vec<_>>();
    unify(equations, &mut inf.answer)?;
    let solved_pre_block_arg_tys = inf.answer.apply_to_vec(&inf.pre_block_arg_tys())?;
    let solved_block_param_tys = inf.answer.apply_to_vec(&inf.block_param_tys())?;
    Ok(MethodCallInf2::new(
        inf,
        solved_pre_block_arg_tys,
        solved_block_param_tys,
    ))
}

pub fn infer_result_ty_with_block(
    mut inf: MethodCallInf2,
    block_ty: &TermTy,
) -> Result<MethodCallInf3> {
    let equations = vec![Equation(
        inf.block_ret_ty.clone(),
        TmpTy::from(&block_ty.tyargs().last().unwrap()),
    )];
    unify(equations, &mut inf.answer)?;
    let solved_block_ret_ty = inf.answer.apply_to(&inf.block_ret_ty)?;
    Ok(MethodCallInf3::with_block(inf, solved_block_ret_ty))
}
