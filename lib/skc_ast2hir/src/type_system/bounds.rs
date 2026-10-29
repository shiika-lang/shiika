//! Check that type arguments satisfy the upper bound of the corresponding
//! type parameter (eg. `Foo<Int>` is an error when `class Foo<T: Ord>`).
//!
//! Checked in two places:
//! - `check_sk_types`: after indexing, checks every type that appears in
//!   the definitions (method signatures, superclass/include clauses and
//!   the bounds themselves).
//! - `check_method_tyargs` / `check_specialization_tyargs`: at expression
//!   conversion, checks method-wise type arguments (explicit or inferred)
//!   and class specialization expressions (eg. `Foo<Int>.new`).
use crate::class_dict::ClassDict;
use crate::error;
use anyhow::Result;
use shiika_ast::LocationSpan;
use shiika_core::ty::{TermTy, TyBody, TyParam};
use skc_error::Label;
use skc_hir::{MethodSignature, SkType};

/// Check all the types in the definitions. Called after indexing so that
/// every type name is resolvable.
pub fn check_sk_types(dict: &ClassDict) -> Result<()> {
    for sk_type in dict.sk_types.types.values() {
        check_typarams_decl(dict, &sk_type.base().typarams)
            .map_err(|e| in_context(e, &format!("type `{}'", sk_type.fullname())))?;
        for (sig, _) in sk_type.base().method_sigs.to_ordered() {
            check_signature(dict, sig)
                .map_err(|e| in_context(e, &format!("method `{}'", sig.fullname)))?;
        }
        if let SkType::Class(sk_class) = sk_type {
            let supers = sk_class.superclass.iter().chain(sk_class.includes.iter());
            for sup in supers {
                check_type(dict, &sup.ty().to_term_ty(), &LocationSpan::internal())
                    .map_err(|e| in_context(e, &format!("type `{}'", sk_type.fullname())))?;
            }
        }
    }
    Ok(())
}

/// Check the upper bounds of typaram declarations (eg. `Array<Int>` of
/// `class Foo<T: Array<Int>>` is itself a type application to check.)
fn check_typarams_decl(dict: &ClassDict, typarams: &[TyParam]) -> Result<()> {
    for tp in typarams {
        check_type(
            dict,
            &tp.upper_bound.to_term_ty(),
            &LocationSpan::internal(),
        )?;
    }
    Ok(())
}

fn check_signature(dict: &ClassDict, sig: &MethodSignature) -> Result<()> {
    check_typarams_decl(dict, &sig.typarams)?;
    for param in &sig.params {
        check_type(dict, &param.ty, &LocationSpan::internal())?;
    }
    check_type(dict, &sig.ret_ty, &LocationSpan::internal())
}

/// Check the method-wise type arguments (explicit or inferred) of a method call.
pub fn check_method_tyargs(
    dict: &ClassDict,
    sig: &MethodSignature,
    tyargs: &[TermTy],
    locs: &LocationSpan,
) -> Result<()> {
    for (tp, tyarg) in sig.typarams.iter().zip(tyargs.iter()) {
        check_tyarg(dict, tp, tyarg, &format!("of `{}'", sig.fullname), locs)?;
    }
    Ok(())
}

/// Check the type arguments of a class specialization (eg. `Foo<Int>`).
pub fn check_specialization_tyargs(
    dict: &ClassDict,
    sk_type: &SkType,
    tyargs: &[TermTy],
    locs: &LocationSpan,
) -> Result<()> {
    for (tp, tyarg) in sk_type.base().typarams.iter().zip(tyargs.iter()) {
        check_tyarg(
            dict,
            tp,
            tyarg,
            &format!("of `{}'", sk_type.fullname()),
            locs,
        )?;
    }
    Ok(())
}

/// Check a type annotation resolved at expression conversion (eg. the type
/// of a block parameter). Do not use this while indexing; the types the
/// annotation refers to may not be indexed yet.
pub fn check_annotation(dict: &ClassDict, ty: &TermTy, locs: &LocationSpan) -> Result<()> {
    check_type(dict, ty, locs)
}

/// Check that every type application in `ty` satisfies the bounds.
fn check_type(dict: &ClassDict, ty: &TermTy, locs: &LocationSpan) -> Result<()> {
    match &ty.body {
        // The bound of a typaram is checked at its declaration
        TyBody::TyPara(_) => Ok(()),
        TyBody::TyRaw(lit_ty) => {
            if lit_ty.type_args.is_empty() {
                return Ok(());
            }
            let Some(sk_type) = dict.find_type(&lit_ty.erasure().to_type_fullname()) else {
                // Unknown type; other checks report this
                return Ok(());
            };
            for (tp, tyarg) in sk_type.base().typarams.iter().zip(lit_ty.type_args.iter()) {
                check_tyarg(
                    dict,
                    tp,
                    tyarg,
                    &format!("of `{}'", sk_type.fullname()),
                    locs,
                )?;
                check_type(dict, tyarg, locs)?;
            }
            Ok(())
        }
    }
}

/// Check that the type argument `tyarg` conforms to the upper bound of `tp`.
fn check_tyarg(
    dict: &ClassDict,
    tp: &TyParam,
    tyarg: &TermTy,
    owner: &str,
    locs: &LocationSpan,
) -> Result<()> {
    if tp.upper_bound.base_name == "Object" {
        // Everything conforms to the default bound
        return Ok(());
    }
    let bound = tp.upper_bound.to_term_ty();
    if dict.conforms(tyarg, &bound) {
        return Ok(());
    }
    let msg = format!(
        "the type argument `{}' does not satisfy the bound `{}' of the type parameter `{}' {}",
        tyarg, &bound, &tp.name, owner
    );
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg.clone()))
    });
    Err(error::type_error(&report))
}

fn in_context(e: anyhow::Error, ctx: &str) -> anyhow::Error {
    error::type_error(&format!("{} (in {})", e, ctx))
}
