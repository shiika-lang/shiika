//! Variance position check.
//!
//! When a type parameter is declared as covariant (`out T`) or contravariant
//! (`in T`), it may only appear in certain positions:
//!
//! - `out T` may only appear in output positions (return types, and covariant
//!   positions of type arguments)
//! - `in T` may only appear in input positions (parameter types, and
//!   contravariant positions of type arguments)
//!
//! Exceptions:
//! - Parameters of `#initialize` (its type arguments are fixed at
//!   construction time, so this is sound; cf. constructor parameters in Scala)
use crate::class_dict::ClassDict;
use crate::error;
use anyhow::Result;
use shiika_ast::LocationSpan;
use shiika_core::ty::{TermTy, TyBody, TyParam, TyParamKind, Variance};
use skc_error::Label;
use skc_hir::MethodSignature;

/// Position in which a type may appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    /// Output position (eg. return types)
    Covariant,
    /// Input position (eg. parameter types)
    Contravariant,
    /// Position where the type argument must match exactly
    Invariant,
}

impl Position {
    /// Compose the current position with the declared variance of the
    /// type parameter whose type argument we are descending into.
    fn compose(self, v: &Variance) -> Position {
        match (self, v) {
            (Position::Invariant, _) => Position::Invariant,
            (_, Variance::Invariant) => Position::Invariant,
            (pos, Variance::Covariant) => pos,
            (Position::Covariant, Variance::Contravariant) => Position::Contravariant,
            (Position::Contravariant, Variance::Contravariant) => Position::Covariant,
        }
    }

    /// Returns true if a typaram of variance `v` may appear in this position.
    fn allows(self, v: &Variance) -> bool {
        match v {
            Variance::Invariant => true,
            Variance::Covariant => self == Position::Covariant,
            Variance::Contravariant => self == Position::Contravariant,
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Position::Covariant => "covariant",
            Position::Contravariant => "contravariant",
            Position::Invariant => "invariant",
        }
    }
}

/// Returns true unless any of `typarams` is co/contravariant.
fn all_invariant(typarams: &[TyParam]) -> bool {
    typarams.iter().all(|t| t.variance == Variance::Invariant)
}

/// Check the types in a method signature.
/// `ast_sig` is used to get the source location of each type.
pub fn check_method_signature(
    dict: &ClassDict,
    class_typarams: &[TyParam],
    ast_sig: &shiika_ast::AstMethodSignature,
    hir_sig: &MethodSignature,
) -> Result<()> {
    if all_invariant(class_typarams) {
        return Ok(());
    }
    if ast_sig.name.0 == "initialize" {
        // Constructor parameters are exempt from the check. However the
        // getters defined from `@`-params (eg. `def initialize(@a: T)`)
        // return the ivar, which is an output position.
        for (ast_param, hir_param) in ast_sig.params.iter().zip(hir_sig.params.iter()) {
            if ast_param.is_iparam {
                check_type(
                    dict,
                    class_typarams,
                    &hir_param.ty,
                    Position::Covariant,
                    &ast_param.typ.locs,
                )?;
            }
        }
        return Ok(());
    }
    for (ast_param, hir_param) in ast_sig.params.iter().zip(hir_sig.params.iter()) {
        check_type(
            dict,
            class_typarams,
            &hir_param.ty,
            Position::Contravariant,
            &ast_param.typ.locs,
        )?;
    }
    if let Some(ret_typ) = &ast_sig.ret_typ {
        check_type(
            dict,
            class_typarams,
            &hir_sig.ret_ty,
            Position::Covariant,
            &ret_typ.locs,
        )?;
    }
    Ok(())
}

/// Check a superclass or included module clause (eg. `Bar<T>` of
/// `class Foo<out T> : Bar<T>`), which is an output position.
pub fn check_supertype(
    dict: &ClassDict,
    class_typarams: &[TyParam],
    ty: &TermTy,
    locs: &LocationSpan,
) -> Result<()> {
    if all_invariant(class_typarams) {
        return Ok(());
    }
    check_type(dict, class_typarams, ty, Position::Covariant, locs)
}

/// Check the type of an enum case field (eg. `V` of `case Some(value: V)`),
/// which is an output position (the field is set only at construction
/// time and read via the getter).
pub fn check_enum_case_ivar(
    dict: &ClassDict,
    enum_typarams: &[TyParam],
    ty: &TermTy,
    locs: &LocationSpan,
) -> Result<()> {
    if all_invariant(enum_typarams) {
        return Ok(());
    }
    check_type(dict, enum_typarams, ty, Position::Covariant, locs)
}

/// Check the types of the ivars of a class.
/// A readonly ivar is an output position (only its getter is public);
/// a mutable ivar is an invariant position.
pub fn check_ivars(
    dict: &ClassDict,
    class_typarams: &[TyParam],
    ivars: &skc_hir::SkIVars,
) -> Result<()> {
    if all_invariant(class_typarams) {
        return Ok(());
    }
    for ivar in ivars.values() {
        let pos = if ivar.readonly {
            Position::Covariant
        } else {
            Position::Invariant
        };
        // No source location available for inferred ivars
        check_type(dict, class_typarams, &ivar.ty, pos, &LocationSpan::internal()).map_err(
            |e| {
                error::program_error(format!("{} (instance variable `{}')", e, &ivar.name))
            },
        )?;
    }
    Ok(())
}

/// Check that co/contravariant typarams in `class_typarams` appear only in
/// valid positions in `ty`.
fn check_type(
    dict: &ClassDict,
    class_typarams: &[TyParam],
    ty: &TermTy,
    pos: Position,
    locs: &LocationSpan,
) -> Result<()> {
    match &ty.body {
        TyBody::TyPara(tpref) => {
            if tpref.kind == TyParamKind::Class {
                if let Some(tp) = class_typarams.iter().find(|t| t.name == tpref.name) {
                    if !pos.allows(&tp.variance) {
                        return Err(variance_error(tp, pos, locs));
                    }
                }
            }
            Ok(())
        }
        TyBody::TyRaw(lit) => {
            let typarams = dict.typarams_of(&lit.erasure().to_type_fullname());
            for (arg, tp) in lit.type_args.iter().zip(typarams.iter()) {
                check_type(dict, class_typarams, arg, pos.compose(&tp.variance), locs)?;
            }
            Ok(())
        }
    }
}

fn variance_error(tp: &TyParam, pos: Position, locs: &LocationSpan) -> anyhow::Error {
    let v = match tp.variance {
        Variance::Covariant => "covariant",
        Variance::Contravariant => "contravariant",
        Variance::Invariant => unreachable!(),
    };
    let msg = format!(
        "{} type parameter `{}' appears in {} position",
        v,
        &tp.name,
        pos.describe()
    );
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg.clone()))
    });
    error::program_error(report)
}
