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
use crate::hir_maker::HirMaker;
use anyhow::Result;
use shiika_core::ty;
use skc_corelib::Corelib;
use skc_hir::Hir;
use skc_mir::LibraryExports;
mod rustlib_methods;

pub fn make_hir(
    ast: shiika_ast::Program,
    corelib: Option<Corelib>,
    imports: &LibraryExports,
) -> Result<Hir> {
    let (core_classes, core_methods) = if let Some(c) = corelib {
        rustlib_methods::mix_with_corelib(c)
    } else {
        (Default::default(), Default::default())
    };
    let class_dict = class_dict::create(&ast, core_classes, &imports.sk_classes)?;

    let mut hir_maker = HirMaker::new(class_dict, &imports.constants);
    hir_maker.define_class_constants();
    let (main_exprs, main_lvars) = hir_maker.convert_toplevel_items(&ast.toplevel_items)?;
    let mut hir = hir_maker.extract_hir(main_exprs, main_lvars);

    // While corelib classes are included in `class_dict`,
    // corelib methods are not. Here we need to add them manually
    hir.add_methods(core_methods);

    Ok(hir)
}

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
            ty::TyParam {
                name: param.name.clone(),
                variance: v,
            }
        })
        .collect::<Vec<_>>()
}
