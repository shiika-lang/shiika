use crate::error::Error;
use crate::hir;
use crate::hir::*;
use crate::ty;
use crate::ty::*;

macro_rules! type_error {
    ( $( $arg:expr ),* ) => ({
        crate::error::type_error(&format!( $( $arg ),* ))
    })
}

pub fn check_return_value(class_dict: &ClassDict, sig: &MethodSignature, ty: &TermTy) -> Result<(), Error> {
    if ty.conforms_to(&sig.ret_ty, class_dict) || sig.ret_ty.is_void_type() {
        Ok(())
    } else {
        Err(type_error!(
            "{} should return {} but returns {}",
            sig.fullname,
            sig.ret_ty.fullname,
            ty.fullname
        ))
    }
}

pub fn check_logical_operator_ty(ty: &TermTy, on: &str) -> Result<(), Error> {
    if *ty == ty::raw("Bool") {
        Ok(())
    } else {
        Err(type_error!("{} must be bool but got {:?}", on, ty.fullname))
    }
}

pub fn check_condition_ty(ty: &TermTy, on: &str) -> Result<(), Error> {
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

pub fn check_reassign_var(orig_ty: &TermTy, new_ty: &TermTy, name: &str) -> Result<(), Error> {
    if orig_ty.equals_to(new_ty) {
        Ok(())
    } else {
        Err(type_error!(
            "variable {} is {:?} but tried to assign a {:?}",
            name,
            orig_ty,
            new_ty
        ))
    }
}

pub fn check_method_args(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_tys: &[&TermTy],
    receiver_hir: &hir::HirExpression,
    arg_hirs: &[hir::HirExpression],
) -> Result<(), Error> {
    check_method_arity(sig, arg_tys, receiver_hir, arg_hirs)?;
    check_arg_types(class_dict, sig, arg_tys, receiver_hir, arg_hirs)?;
    Ok(())
}

fn check_method_arity(
    sig: &MethodSignature,
    arg_tys: &[&TermTy],
    receiver_hir: &hir::HirExpression,
    arg_hirs: &[hir::HirExpression],
) -> Result<(), Error> {
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

fn check_arg_types(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_tys: &[&TermTy],
    receiver_hir: &hir::HirExpression,
    arg_hirs: &[hir::HirExpression],
) -> Result<(), Error> {
    for (param, arg_ty) in sig.params.iter().zip(arg_tys.iter()) {
        let a = arg_ty.upper_bound();
        let p = param.ty.upper_bound();
        if !a.conforms_to(&p, class_dict) {
            return Err(type_error!(
                "{} takes {} but got {} (receiver: {:?}, args: {:?})",
                sig.fullname,
                param.ty.fullname,
                arg_ty.fullname,
                receiver_hir,
                arg_hirs
            ))
        }
    }
    Ok(())
}
