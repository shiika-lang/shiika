mod term_ty;
mod lit_ty;
mod typaram;
pub use crate::ty::term_ty::{TermTy, TyParamKind};
pub use crate::ty::term_ty::TyBody; // REFACTOR: should be private
pub use crate::ty::lit_ty::LitTy;
pub use crate::ty::typaram::{TyParam, Variance};
use crate::names::*;
use crate::ty;

pub fn new(
    base_name_: impl Into<String>,
    type_args: Vec<TermTy>,
    is_meta: bool
) -> TermTy {
    let base_name = base_name_.into();
    debug_assert!(!base_name.is_empty());
    debug_assert!(!base_name.starts_with("Meta:"));
    debug_assert!(!base_name.contains('<'));
    let fullname = ClassFullname::new(
        format!("{}{}", &base_name, &tyargs_str(&type_args)),
        is_meta
    );
    TermTy {
        fullname,
        body: term_ty::TyBody::TyRaw(LitTy::new(base_name, type_args, is_meta))
    }
}

pub fn nonmeta(names: &[String], args: Vec<TermTy>) -> TermTy {
    ty::new(&names.join("::"), args, false)
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

/// Create the type of return value of `.new` method of the class
pub fn return_type_of_new(classname: &ClassFullname, typarams: &[TyParam]) -> TermTy {
    if typarams.is_empty() {
        ty::raw(&classname.0)
    } else {
        let args = typarams
            .iter()
            .enumerate()
            .map(|(i, t)| typaram(&t.name, TyParamKind::Class, i))
            .collect::<Vec<_>>();
        ty::spe(&classname.0, args)
    }
}

/// Shortcut for Array<T>
pub fn ary(type_arg: TermTy) -> TermTy {
    spe("Array", vec![type_arg])
}

pub fn typaram(name: impl Into<String>, kind: TyParamKind, idx: usize) -> TermTy {
    let s = name.into();
    TermTy {
        // TODO: s is not a class name. `fullname` should be just a String
        fullname: class_fullname(s.clone()),
        body: term_ty::TyBody::TyParamRef {
            kind,
            name: s,
            idx,
            upper_bound: Box::new(ty::raw("Object")),
            lower_bound: Box::new(ty::raw("Never")),
        },
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

