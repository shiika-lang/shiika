mod erasure;
mod lit_ty;
mod term_ty;
mod typaram;
mod typaram_ref;
use crate::names::*;
use crate::ty;
pub use crate::ty::erasure::Erasure;
pub use crate::ty::lit_ty::LitTy;
pub use crate::ty::term_ty::TermTy;
pub use crate::ty::term_ty::TyBody; // REFACTOR: should be private
pub use crate::ty::typaram::{TyParam, Variance};
pub use crate::ty::typaram_ref::{TyParamKind, TyParamRef};

pub fn new(base_name_: impl Into<String>, type_args: Vec<TermTy>, is_meta: bool) -> TermTy {
    let base_name = base_name_.into();
    debug_assert!(!base_name.is_empty());
    debug_assert!(!base_name.starts_with("Meta:"));
    debug_assert!(!base_name.contains('<'));
    let fullname = TypeFullname::new(
        format!("{}{}", &base_name, &tyargs_str(&type_args)),
        is_meta,
    );
    TermTy {
        fullname,
        body: term_ty::TyBody::TyRaw(LitTy::new(base_name, type_args, is_meta)),
    }
}

pub fn nonmeta(name: impl Into<String>, args: Vec<TermTy>) -> TermTy {
    ty::new(name, args, false)
}

/// Returns the type of instances of the class
pub fn raw(fullname_: impl Into<String>) -> TermTy {
    let fullname = fullname_.into();
    // Usually this is `false`; the only exception is the class `Metaclass`
    let meta = fullname == "Metaclass";
    new(fullname, Default::default(), meta)
}

/// Returns the type of the class object
pub fn meta(base_fullname_: impl Into<String>) -> TermTy {
    new(base_fullname_, Default::default(), true)
}

pub fn spe(base_name_: impl Into<String>, type_args: Vec<TermTy>) -> TermTy {
    new(base_name_, type_args, false)
}

pub fn spe_meta(base_name_: impl Into<String>, type_args: Vec<TermTy>) -> TermTy {
    new(base_name_, type_args, true)
}

pub fn typarams_to_tyargs(typarams: &[TyParam]) -> Vec<TermTy> {
    typarams_to_typaram_refs(typarams, TyParamKind::Class, false)
        .into_iter()
        .map(|tpref| tpref.into_term_ty())
        .collect()
}

pub fn typarams_to_typaram_refs(
    typarams: &[TyParam],
    kind: TyParamKind,
    as_class: bool,
) -> Vec<TyParamRef> {
    typarams
        .iter()
        .enumerate()
        .map(|(i, t)| TyParamRef {
            kind: kind.clone(),
            name: t.name.clone(),
            idx: i,
            upper_bound: t.upper_bound.clone(),
            lower_bound: t.lower_bound.clone(),
            as_class,
        })
        .collect()
}

/// Shortcut for Array<T>
pub fn ary(type_arg: TermTy) -> TermTy {
    spe("Array", vec![type_arg])
}

// Note: this should eventually removed when upper/lower bound is fully implemented
pub fn typaram_ref(name: impl Into<String>, kind: TyParamKind, idx: usize) -> TyParamRef {
    TyParamRef {
        kind,
        name: name.into(),
        idx,
        upper_bound: LitTy::raw("Object"),
        lower_bound: LitTy::raw("Never"),
        as_class: false,
    }
}

/// Returns "" if the argument is empty.
/// Returns a string like "<A,B,C>" otherwise.
fn tyargs_str(type_args: &[TermTy]) -> String {
    if type_args.is_empty() {
        "".to_string()
    } else {
        let s = type_args
            .iter()
            .map(|x| x.fullname.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("<{}>", &s)
    }
}

pub fn fn_ty(arg_tys: Vec<TermTy>, ret_ty: TermTy) -> TermTy {
    let name = format!("Fn{}", arg_tys.len());
    let mut type_args = arg_tys;
    type_args.push(ret_ty);
    ty::spe(name, type_args)
}
