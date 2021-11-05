#![feature(backtrace)]
mod accessors;
pub mod class_dict;
mod convert_exprs;
mod ctx_stack;
mod error;
mod hir_maker;
mod hir_maker_context;
mod method_dict;
mod pattern_match;
mod type_checking;
use shiika_ast;
use shiika_core::ty;

pub fn parse_typarams(typarams: &[shiika_ast::AstTyParam]) -> Vec<ty::TyParam> {
    typarams
        .iter()
        .map(|param| {
            let v = match &param.variance {
                shiika_ast::AstVariance::Invariant => ty::Variance::Invariant,
                shiika_ast::AstVariance::Covariant => ty::Variance::Covariant,
                shiika_ast::AstVariance::Contravariant => ty::Variance::Contravariant,
            };
            ty::TyParam {
                name: param.name.clone(),
                variance: v,
            }
        })
        .collect::<Vec<_>>()
}
