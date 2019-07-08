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
        Err(type_error!("if condition must be bool but got {:?}", ty.fullname))
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
