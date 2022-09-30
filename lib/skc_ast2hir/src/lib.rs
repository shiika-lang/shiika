mod accessors;
pub mod class_dict;
mod convert_exprs;
mod ctx_stack;
mod error;
mod hir_maker;
mod hir_maker_context;
mod method_dict;
mod pattern_match;
mod type_system;
use crate::class_dict::type_index;
use crate::hir_maker::HirMaker;
use anyhow::Result;
use shiika_ast::LocationSpan;
use shiika_core::{names::*, ty, ty::*};
use skc_corelib::Corelib;
use skc_hir::{Hir, HirExpression};
use skc_mir::LibraryExports;
mod rustlib_methods;

pub fn make_hir(ast: shiika_ast::Program, imports: &LibraryExports) -> Result<Hir> {
    let defs = ast.defs();
    let class_dict = class_dict::create(&defs, &imports.sk_types)?;

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
    let class_dict = class_dict::create_for_corelib(
        &defs,
        &dummy_imports,
        corelib.sk_types,
        type_index,
        &rust_method_sigs,
    )?;

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
            ty::TyParam {
                name: param.name.clone(),
                variance: v,
            }
        })
        .collect::<Vec<_>>()
}

/// Build a HirExpression which evaluates to `ty`
/// eg. `Foo.<>([Bool, Int])` if `ty` is `TermTy(Foo<Bool, Int>)`
pub fn class_expr(mk: &mut HirMaker, ty: &TermTy) -> HirExpression {
    match &ty.body {
        TyBody::TyRaw(LitTy {
            base_name,
            type_args,
            is_meta,
        }) => {
            debug_assert!(!is_meta);
            let base = Hir::const_ref(
                ty::meta(base_name),
                toplevel_const(base_name),
                LocationSpan::todo(),
            );
            if type_args.is_empty() {
                base
            } else {
                let tyargs = type_args
                    .iter()
                    .map(|t| Hir::bit_cast(ty::raw("Class"), class_expr(mk, t)))
                    .collect();
                call_class_specialize(mk, tyargs, base_name, base)
            }
        }
        TyBody::TyPara(typaram_ref) => {
            let ref2 = typaram_ref.as_class();
            Hir::tvar_ref(
                ref2.to_term_ty(),
                ref2,
                mk.ctx_stack.self_ty(),
                LocationSpan::todo(),
            )
        }
    }
}

/// Generate call to `Class#<>`
fn call_class_specialize(
    mk: &mut HirMaker,
    mut tyargs: Vec<HirExpression>,
    base_name: &str,
    base: HirExpression,
) -> HirExpression {
    if tyargs.len() == 1 {
        // Workaround for bootstrap problem of arrays.
        // `_specialize1` is the same as `<>` except it accepts only one
        // type argument and therefore does not need to create an array.
        Hir::method_call(
            ty::meta(base_name),
            base,
            method_fullname_raw("Class", "_specialize1"),
            vec![tyargs.remove(0)],
        )
    } else {
        Hir::method_call(
            ty::meta(base_name),
            base,
            method_fullname_raw("Class", "<>"),
            vec![mk.create_array_instance(tyargs, LocationSpan::todo())],
        )
    }
}
