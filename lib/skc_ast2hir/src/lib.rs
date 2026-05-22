mod accessors;
pub mod class_dict;
mod convert_exprs;
mod ctx_stack;
mod error;
pub mod hir_maker;
mod hir_maker_context;
mod method_dict;
mod pattern_match;
mod type_inference;
pub mod type_system;
pub use crate::class_dict::type_index;
use shiika_core::ty;

/// Convert AstTyParam to TyParam
fn parse_typarams(typarams: &[shiika_ast::AstTyParam]) -> Vec<ty::TyParam> {
    typarams
        .iter()
        .map(|param| {
            let v = match &param.variance {
                shiika_ast::AstVariance::Invariant => ty::Variance::Invariant,
                shiika_ast::AstVariance::Covariant => ty::Variance::Covariant,
                shiika_ast::AstVariance::Contravariant => ty::Variance::Contravariant,
            };
            ty::TyParam::new(param.name.clone(), v)
        })
        .collect::<Vec<_>>()
}
