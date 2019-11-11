use crate::error::Error;
use crate::ty;
use crate::ty::*;

macro_rules! type_error {
    ( $( $arg:expr ),* ) => ({
        crate::error::type_error(&format!( $( $arg ),* ))
    })
}

pub fn check_return_value(sig: &MethodSignature, ty: &TermTy) -> Result<(), Error> {
    if ty.conforms_to(&sig.ret_ty) || sig.ret_ty.is_void_type() {
        Ok(())
    }
    else {
        Err(type_error!("{} should return {} but returns {}",
                        sig.fullname, sig.ret_ty.fullname, ty.fullname))
    }
}

pub fn check_condition_ty(ty: &TermTy, on: &str) -> Result<(), Error> {
    if *ty == ty::raw("Bool") {
        Ok(())
    }
    else {
        Err(type_error!("{} condition must be bool but got {:?}", on, ty.fullname))
    }
}

pub fn check_reassign_var(orig_ty: &TermTy, new_ty: &TermTy, name: &str) -> Result<(), Error> {
    if orig_ty.equals_to(new_ty) {
        Ok(())
    }
    else {
        Err(type_error!("variable {} is {:?} but tried to assign a {:?}", name, orig_ty, new_ty))
    }
}

pub fn check_method_args(sig: &MethodSignature, arg_tys: &Vec<&TermTy>) -> Result<(), Error> {
    if sig.params.len() != arg_tys.len() {
        return Err(type_error!("{} takes {} args but got {}", sig.fullname, sig.params.len(), arg_tys.len()));
    }

    sig.params.iter().zip(arg_tys.iter()).try_for_each(|(param, arg_ty)| {
        if arg_ty.conforms_to(&param.ty) {
            Ok(())
        }
        else {
            Err(type_error!("{} takes {} but got {}",
                            sig.fullname, param.ty.fullname, arg_ty.fullname))
        }
    })?;

    Ok(())
}
