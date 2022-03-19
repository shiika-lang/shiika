#![feature(backtrace)]
mod accessors;
mod convert_exprs;
mod ctx_stack;
mod error;
mod hir_maker;
mod hir_maker_context;
mod method_dict;
pub mod module_dict;
mod pattern_match;
mod type_checking;
use crate::hir_maker::HirMaker;
use anyhow::Result;
use shiika_core::{names::*, ty, ty::*};
use skc_corelib::Corelib;
use skc_hir::{Hir, HirExpression};
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
    let module_dict = module_dict::create(&ast, core_classes, &imports.sk_modules)?;

    let mut hir_maker = HirMaker::new(module_dict, &imports.constants);
    hir_maker.define_class_constants();
    let (main_exprs, main_lvars) = hir_maker.convert_toplevel_items(&ast.toplevel_items)?;
    let mut hir = hir_maker.extract_hir(main_exprs, main_lvars);

    // While corelib classes are included in `module_dict`,
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
            let base = Hir::const_ref(ty::meta(base_name), toplevel_const(base_name));
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
            Hir::tvar_ref(ref2.to_term_ty(), ref2, mk.ctx_stack.self_ty())
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
            vec![mk.create_array_instance(tyargs)],
        )
    }
}
