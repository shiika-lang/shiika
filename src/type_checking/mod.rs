use crate::error::Error;
use crate::ty;
use crate::ty::*;

macro_rules! type_error {
    ( $( $arg:expr ),* ) => ({
        crate::error::type_error(&format!( $( $arg ),* ))
    })
}

pub fn check_if_condition_ty(ty: &TermTy) -> Result<(), Error> {
    if *ty == ty::raw("Bool") {
        Ok(())
    }
    else {
        Err(type_error!("if condition must be bool but got {:?}", ty.class_fullname()))
    }
}

pub fn check_method_args(sig: &MethodSignature, arg_tys: &Vec<&TermTy>) -> Result<(), Error> {
    if sig.arg_tys.len() != arg_tys.len() {
        return Err(type_error!("{} takes {} args but got {}", sig.fullname, sig.arg_tys.len(), arg_tys.len()));
    }

    sig.arg_tys.iter().zip(arg_tys.iter()).try_for_each(|(param_ty, arg_ty)| {
        if arg_ty.conforms_to(param_ty) {
            Ok(())
        }
        else {
            Err(type_error!("{} takes {} but got {}",
                            sig.fullname, param_ty.class_fullname(), arg_ty.class_fullname()))
        }
    })?;

    Ok(())
}
