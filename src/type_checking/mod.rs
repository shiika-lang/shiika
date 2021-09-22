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

pub fn check_return_value(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    ty: &TermTy,
) -> Result<(), Error> {
    if sig.ret_ty.is_void_type() || class_dict.conforms(ty, &sig.ret_ty) {
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

pub fn merge_ifs(
    mut then_hirs: HirExpressions,
    mut else_hirs: HirExpressions,
    class_dict: &ClassDict,
) -> (TermTy, HirExpressions, HirExpressions) {
    let ty = if then_hirs.ty.is_never_type() {
        else_hirs.ty.clone()
    } else if else_hirs.ty.is_never_type() {
        then_hirs.ty.clone()
    } else if then_hirs.ty.is_void_type() {
        else_hirs.voidify();
        ty::raw("Void")
    } else if else_hirs.ty.is_void_type() {
        then_hirs.voidify();
        ty::raw("Void")
    } else {
        let ty = class_dict.nearest_common_ancestor(&then_hirs.ty, &else_hirs.ty);
        if !then_hirs.ty.equals_to(&ty) {
            then_hirs = then_hirs.bitcast_to(ty.clone());
        }
        if !else_hirs.ty.equals_to(&ty) {
            else_hirs = else_hirs.bitcast_to(ty.clone());
        }
        ty
    };
    (ty, then_hirs, else_hirs)
}

/// Check the type of the argument of `return`
pub fn check_return_arg_type(
    class_dict: &ClassDict,
    return_arg_ty: &TermTy,
    method_sig: &MethodSignature,
) -> Result<(), Error> {
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

/// Check argument types of a method call
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

/// Check number of method call args
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

/// Check types of method call args
fn check_arg_types(
    class_dict: &ClassDict,
    sig: &MethodSignature,
    arg_tys: &[&TermTy],
    receiver_hir: &hir::HirExpression,
    arg_hirs: &[hir::HirExpression],
) -> Result<(), Error> {
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
