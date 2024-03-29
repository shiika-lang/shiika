use crate::class_dict::ClassDict;
use crate::convert_exprs::block::BlockTaker;
use crate::error;
use crate::error::type_error;
use anyhow::Result;
use shiika_ast::LocationSpan;
use shiika_core::{ty, ty::*};
use skc_error::{self, Label};
use skc_hir::*;

macro_rules! type_error {
    ( $( $arg:expr ),* ) => ({
        type_error(&format!( $( $arg ),* ))
    })
}

pub fn check_return_value(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    body_exprs: &HirExpression,
) -> Result<()> {
    let ty = &body_exprs.ty;
    if sig.ret_ty.is_void_type() {
        return Ok(());
    }
    let want = match &sig.ret_ty.body {
        TyBody::TyPara(TyParamRef { lower_bound, .. }) => {
            // To avoid errors like this. (I'm not sure this is the right way;
            // looks ad-hoc)
            // > TypeError: Maybe#expect should return TermTy(TyParamRef(V 0C)) but returns TermTy(TyParamRef(V 0C))
            if ty.equals_to(&sig.ret_ty) {
                return Ok(());
            }
            lower_bound.to_term_ty()
        }
        _ => sig.ret_ty.clone(),
    };
    if class_dict.conforms(ty, &want) {
        return Ok(());
    }

    let main_msg = format!(
        "{} should return {} but returns {}",
        sig.fullname, sig.ret_ty, ty
    );
    let sub_msg = format!("This evaluates to {} but should be {}", ty, sig.ret_ty);
    let locs = &body_exprs.last_expr().locs;
    let report = skc_error::build_report(main_msg, locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(sub_msg))
    });
    Err(type_error(report))
}

pub fn check_logical_operator_ty(ty: &TermTy, on: &str) -> Result<()> {
    if *ty == ty::raw("Bool") {
        Ok(())
    } else {
        Err(type_error!("{} must be bool but got {:?}", on, ty.fullname))
    }
}

pub fn check_condition_ty(ty: &TermTy, on: &str) -> Result<()> {
    if *ty == ty::raw("Bool") {
        Ok(())
    } else {
        Err(type_error!(
            "{} condition must be bool but got {:?}",
            on,
            ty.fullname
        ))
    }
}

pub fn check_if_body_ty(
    class_dict: &ClassDict,
    then_ty: &TermTy,
    then_locs: LocationSpan,
    else_ty: &TermTy,
    else_locs: LocationSpan,
) -> Result<TermTy> {
    if let Some(ty) = class_dict.nearest_common_ancestor(then_ty, else_ty) {
        Ok(ty)
    } else {
        Err(error::if_clauses_type_mismatch(
            then_ty, else_ty, then_locs, else_locs,
        ))
    }
}

/// Check the type of the argument of `return`
pub fn check_return_arg_type(
    class_dict: &ClassDict,
    return_arg_ty: &TermTy,
    sig: &MethodSignature,
    locs: &LocationSpan,
) -> Result<()> {
    if class_dict.conforms(return_arg_ty, &sig.ret_ty) {
        return Ok(());
    }
    let main_msg = format!(
        "method {} should return {} but returns {}",
        sig.fullname, sig.ret_ty, return_arg_ty
    );
    let sub_msg = format!(
        "This returns {} but should be {}",
        return_arg_ty, sig.ret_ty
    );
    let report = skc_error::build_report(main_msg, locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(sub_msg))
    });
    Err(type_error(report))
}

pub fn invalid_reassign_error(orig_ty: &TermTy, new_ty: &TermTy, name: &str) -> anyhow::Error {
    type_error!(
        "variable {} is {:?} but tried to assign a {:?}",
        name,
        orig_ty,
        new_ty
    )
}

/// Check argument types of a method call
pub fn check_method_args(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    _receiver_hir: &HirExpression,
    arg_hirs: &[HirExpression],
    param_types: &[TermTy],
) -> Result<()> {
    let mut result = check_method_arity(sig, arg_hirs);
    if result.is_ok() {
        result = check_arg_types(class_dict, sig, arg_hirs, param_types);
    }

    if result.is_err() {
        // Remove this when shiika can show the location in the .sk
        //dbg!(&receiver_hir);
        //dbg!(&sig.fullname);
        //dbg!(&arg_hirs);
    }
    result
}

/// Check number of method call args
fn check_method_arity(sig: &MethodSignature, arg_hirs: &[HirExpression]) -> Result<()> {
    if sig.params.len() != arg_hirs.len() {
        return Err(type_error!(
            "{} takes {} args but got {}",
            sig.fullname,
            sig.params.len(),
            arg_hirs.len()
        ));
    }
    Ok(())
}

/// Check types of method call args
#[allow(clippy::needless_range_loop)]
fn check_arg_types(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_hirs: &[HirExpression],
    param_types: &[TermTy],
) -> Result<()> {
    for i in 0..sig.params.len() {
        let param = &sig.params[i];
        let arg_hir = &arg_hirs[i];
        let param_ty = &param_types[i];
        check_arg_type(class_dict, sig, arg_hir, param, param_ty)?;
    }
    Ok(())
}

/// Check types of method call args
fn check_arg_type(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_hir: &HirExpression,
    param: &MethodParam,
    param_ty: &TermTy,
) -> Result<()> {
    let arg_ty = &arg_hir.ty;
    if class_dict.conforms(arg_ty, param_ty) {
        return Ok(());
    }

    let msg = format!(
        "the argument `{}' of `{}' should be {} but got {}",
        param.name, sig.fullname, param_ty, arg_ty
    );
    let locs = &arg_hir.locs;
    let report = skc_error::build_report(msg, locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(&arg_ty))
    });
    Err(type_error(report))
}

/// Check number of block parameters
pub fn check_block_arity(
    block_taker: &BlockTaker, // for error message
    expected_arity: usize,
    params: &[shiika_ast::BlockParam],
) -> Result<()> {
    if params.len() == expected_arity {
        return Ok(());
    }

    let msg = format!(
        "the block of {} takes {} args but got {}",
        block_taker,
        expected_arity,
        params.len()
    );
    let locs = &block_taker.locs();
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    Err(type_error(report))
}

pub fn check_class_specialization(
    class: &SkType,
    given_tyargs: &[HirExpression],
    locs: &LocationSpan,
) -> Result<()> {
    let expected = class.base().typarams.len();
    if expected == given_tyargs.len() {
        return Ok(());
    }

    let main_msg = format!(
        "the type {} takes {} type arg(s) but got {}",
        class.fullname(),
        expected,
        given_tyargs.len()
    );
    let sub_msg = format!("should take {} type arg(s)", expected);
    let report = skc_error::build_report(main_msg, locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(sub_msg))
    });
    Err(type_error(report))
}
