use crate::class_dict::ClassDict;
use crate::convert_exprs::block::BlockTaker;
use crate::error::type_error;
use anyhow::Result;
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
    ty: &TermTy,
) -> Result<()> {
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
        Ok(())
    } else {
        Err(type_error!(
            "{} should return {:?} but returns {:?}",
            sig.fullname,
            sig.ret_ty,
            ty
        ))
    }
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

pub fn check_if_body_ty(opt_ty: Option<TermTy>) -> Result<TermTy> {
    match opt_ty {
        Some(ty) => Ok(ty),
        None => Err(type_error!("if clauses type mismatch")),
    }
}

/// Check the type of the argument of `return`
pub fn check_return_arg_type(
    class_dict: &ClassDict,
    return_arg_ty: &TermTy,
    method_sig: &MethodSignature,
) -> Result<()> {
    if class_dict.conforms(return_arg_ty, &method_sig.ret_ty) {
        Ok(())
    } else {
        Err(type_error!(
            "method {} should return {} but returns {}",
            &method_sig.fullname,
            &method_sig.ret_ty,
            &return_arg_ty
        ))
    }
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
    receiver_hir: &HirExpression,
    arg_hirs: &[HirExpression],
) -> Result<()> {
    let mut result = check_method_arity(sig, arg_hirs);
    if result.is_ok() {
        result = check_arg_types(class_dict, sig, arg_hirs);
    }

    if result.is_err() {
        // Remove this when shiika can show the location in the .sk
        dbg!(&receiver_hir);
        dbg!(&sig.fullname);
        dbg!(&arg_hirs);
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
fn check_arg_types(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_hirs: &[HirExpression],
) -> Result<()> {
    for (param, arg_hir) in sig.params.iter().zip(arg_hirs.iter()) {
        check_arg_type(class_dict, sig, arg_hir, param)?;
    }
    Ok(())
}

/// Check types of method call args
fn check_arg_type(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_hir: &HirExpression,
    param: &MethodParam,
) -> Result<()> {
    let arg_ty = &arg_hir.ty;
    if class_dict.conforms(arg_ty, &param.ty) {
        return Ok(());
    }

    let msg = format!(
        "the argument `{}' of `{}' should be {} but got {}",
        param.name, sig.fullname, param.ty.fullname, arg_ty.fullname
    );
    let locs = &arg_hir.locs;
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(&arg_hir.ty))
    });
    Err(type_error(report))
}

/// Check number of block parameters
pub fn check_block_arity(
    block_taker: &BlockTaker,
    params: &[shiika_ast::BlockParam],
) -> Result<()> {
    let expected = match block_taker {
        BlockTaker::Method { sig, .. } => sig.block_ty().unwrap().len() - 1,
        BlockTaker::Function { fn_ty, .. } => fn_ty.fn_x_info().unwrap().len() - 1,
    };
    if params.len() == expected {
        return Ok(());
    }

    let msg = format!(
        "the block of {} takes {} args but got {}",
        block_taker,
        expected,
        params.len()
    );
    let locs = &block_taker.locs();
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    Err(type_error(report))
}
