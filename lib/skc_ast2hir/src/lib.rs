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
mod type_system;
pub use crate::class_dict::type_index;
use crate::hir_maker::HirMaker;
use anyhow::Result;
use shiika_core::ty;
use skc_corelib::Corelib;
use skc_hir::Hir;
use skc_mir::LibraryExports;
mod rustlib_methods;

pub fn make_hir(ast: shiika_ast::Program, imports: &LibraryExports) -> Result<Hir> {
    let defs = ast.defs();
    let type_index = type_index::create(&defs, &Default::default(), &imports.sk_types);
    let class_dict = class_dict::create(&defs, type_index, &imports.sk_types)?;

    let mut hir_maker = HirMaker::new(class_dict, &imports.constants);
    hir_maker.define_class_constants()?;
    let (main_exprs, main_lvars) = hir_maker.convert_toplevel_items(ast.toplevel_items)?;
    let hir = hir_maker.extract_hir(main_exprs, main_lvars);

    Ok(hir)
}

pub fn make_corelib_hir(
    // ast of builtin/*.sk
    ast: shiika_ast::Program,
    corelib: Corelib,
) -> Result<Hir> {
    let defs = ast.defs();
    // TODO: Remove this. (`imports` is a reference because it is used for building
    // mir too. But I think we can put `imports` into hir)
    let dummy_imports = Default::default();
    let dummy_constants = Default::default();
    // Collect types defined in .sk, so that...
    let type_index = type_index::create(&defs, &corelib.sk_types, &Default::default());
    // they can be referred in the signatures of methods written with Rust.
    let rust_method_sigs = rustlib_methods::create_method_sigs(&type_index);
    let class_dict =
        class_dict::create_for_corelib(&defs, &dummy_imports, corelib.sk_types, type_index)?;

    let mut hir_maker = HirMaker::new(class_dict, &dummy_constants);
    hir_maker.define_class_constants()?;
    let (main_exprs, main_lvars) = hir_maker.convert_toplevel_items(ast.toplevel_items)?;
    let mut hir = hir_maker.extract_hir(main_exprs, main_lvars);
    hir.add_methods(rustlib_methods::make_sk_methods(rust_method_sigs));

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
            ty::TyParam::new(param.name.clone(), v)
        })
        .collect::<Vec<_>>()
}
