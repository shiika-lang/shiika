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
use crate::class_dict::ClassDict;
use anyhow::Result;
use shiika_ast;
use shiika_core::{ty, names::*};
use skc_corelib::Corelib;
use skc_hir::{Hir, SkMethod, SkMethods, SkMethodBody};
use skc_hir2ll::library::LibraryExports;
use std::collections::HashMap;

pub fn make_hir(
    ast: shiika_ast::Program,
    corelib: Option<Corelib>,
    imports: &LibraryExports,
    rustlib: &[(ClassFullname, shiika_ast::AstMethodSignature)],
) -> Result<Hir> {
    let (core_classes, core_methods) = if let Some(c) = corelib {
        (c.sk_classes, c.sk_methods)
    } else {
        (Default::default(), Default::default())
    };
    let class_dict = class_dict::create(&ast, core_classes, &imports.sk_classes)?;
    let rustlib_methods = parse_rustlib_methods(rustlib, &class_dict)?;

    let mut hir_maker = HirMaker::new(class_dict, &imports.constants);
    hir_maker.define_class_constants();
    let (main_exprs, main_lvars) = hir_maker.convert_toplevel_items(&ast.toplevel_items)?;
    let mut hir = hir_maker.extract_hir(main_exprs, main_lvars);

    // While corelib classes are included in `class_dict`,
    // corelib/rustlib methods are not. Here we need to add them manually
    hir.add_methods(core_methods);
    hir.add_methods(rustlib_methods);

    Ok(hir)
}

fn parse_rustlib_methods(
    rustlib: &[(ClassFullname, shiika_ast::AstMethodSignature)],
    class_dict: &ClassDict,
) -> Result<SkMethods>  {
    let mut lib = HashMap::new();
    for (classname, ast_sig) in rustlib {
        let class_typarams = &class_dict.get_class(&classname).typarams;
        let hir_sig = class_dict.create_signature(
            &Namespace::root(),
            classname,
            ast_sig,
            class_typarams)?;
        let method = SkMethod {
            signature: hir_sig,
            body: SkMethodBody::RustLib,
            lvars: Default::default(),
        };
        if !lib.contains_key(classname) {
            lib.insert(classname.clone(), vec![]);
        }
        let v = lib.get_mut(&classname).unwrap();
        v.push(method);
    }
    Ok(lib)
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
