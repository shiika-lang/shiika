use crate::error;
use crate::ty;
use crate::ty::*;

pub fn check_if_condition_ty(ty: &TermTy) -> Result<(), error::Error> {
    if *ty == ty::raw("Bool") {
        Ok(())
    }
    else {
        Err(error::type_error(&format!("if condition must be bool but got {:?}", ty.class_fullname())))
    }
}
