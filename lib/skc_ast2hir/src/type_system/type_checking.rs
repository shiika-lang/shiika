use crate::class_dict::ClassDict;
use crate::error::type_error;
use anyhow::Result;
use ariadne::{Label, Report, ReportKind, Source};
use shiika_core::{ty, ty::*};
use skc_hir::*;
use std::fs;

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
    let path = format!("{}", locs.filepath.display()); // ariadne 0.1.5 needs Id: Display (zesterer/ariadne#12)
    if path.is_empty() {
        return Err(type_error(msg));
    }
    let span = (&path, locs.begin.pos..locs.end.pos);
    let src = Source::from(fs::read_to_string(&*locs.filepath).unwrap_or_default());
    let mut report = vec![];
    Report::build(ReportKind::Error, &path, locs.begin.pos)
        .with_message(msg.clone())
        .with_label(Label::new(span).with_message(&arg_hir.ty))
        .finish()
        .write((&path, src), &mut report)
        .unwrap();
    return Err(type_error(String::from_utf8_lossy(&report).to_string()));
}
