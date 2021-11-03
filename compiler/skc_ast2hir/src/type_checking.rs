use crate::error;
use anyhow::Result;
use shiika_core::{ty, ty::*};
use skc_hir2ll::{hir, hir::*};

macro_rules! type_error {
    ( $( $arg:expr ),* ) => ({
        crate::error::type_error(&format!( $( $arg ),* ))
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
        TyBody::TyParamRef { lower_bound, .. } => {
            // To avoid errors like this. (I'm not sure this is the right way;
            // looks ad-hoc)
            // > TypeError: Maybe#expect should return TermTy(TyParamRef(V 0C)) but returns TermTy(TyParamRef(V 0C))
            if ty.equals_to(&sig.ret_ty) {
                return Ok(());
            }
            lower_bound
        }
        _ => &sig.ret_ty,
    };
    if class_dict.conforms(ty, want) {
        Ok(())
    } else {
        Err(type_error!(
            "{} should return {} but returns {}",
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

pub fn invalid_reassign_error(orig_ty: &TermTy, new_ty: &TermTy, name: &str) -> Error {
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
    arg_tys: &[&TermTy],
    receiver_hir: &hir::HirExpression,
    arg_hirs: &[hir::HirExpression],
) -> Result<()> {
    check_method_arity(sig, arg_tys, receiver_hir, arg_hirs)?;
    check_arg_types(class_dict, sig, arg_tys, receiver_hir, arg_hirs)?;
    Ok(())
}

/// Check number of method call args
fn check_method_arity(
    sig: &MethodSignature,
    arg_tys: &[&TermTy],
    receiver_hir: &hir::HirExpression,
    arg_hirs: &[hir::HirExpression],
) -> Result<()> {
    if sig.params.len() != arg_tys.len() {
        return Err(type_error!(
            "{} takes {} args but got {} (receiver: {:?}, args: {:?})",
            sig.fullname,
            sig.params.len(),
            arg_tys.len(),
            receiver_hir,
            arg_hirs
        ));
    }
    Ok(())
}

/// Check types of method call args
fn check_arg_types(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_tys: &[&TermTy],
    receiver_hir: &hir::HirExpression,
    arg_hirs: &[hir::HirExpression],
) -> Result<()> {
    for (param, arg_ty) in sig.params.iter().zip(arg_tys.iter()) {
        if !class_dict.conforms(arg_ty, &param.ty) {
            return Err(type_error!(
                "the argument `{}' of `{}' should be {} but got {} (receiver: {:?}, args: {:?})",
                param.name,
                sig.fullname,
                param.ty.fullname,
                arg_ty.fullname,
                receiver_hir,
                arg_hirs
            ));
        }
    }
    Ok(())
}
