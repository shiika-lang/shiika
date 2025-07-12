use crate::type_index::TypeIndex;
use shiika_ast::{AstMethodSignature, UnresolvedTypeName};
use shiika_core::names::ClassFullname;
use shiika_core::{names::method_fullname, ty, ty::TermTy};
use skc_corelib::{self};
use skc_hir::*;
use std::collections::HashMap;

/// Convert signatures of Rust methods to SkMethods
pub fn make_sk_methods(sigs: Vec<MethodSignature>) -> SkMethods {
    let mut sk_methods = HashMap::new();
    for signature in sigs {
        let typename = signature.fullname.type_name.clone();
        let method = SkMethod::simple(signature, SkMethodBody::RustLib);
        let v: &mut Vec<SkMethod> = sk_methods.entry(typename).or_default();
        v.push(method);
    }
    sk_methods
}

pub fn create_method_sigs(type_index: &TypeIndex) -> Vec<MethodSignature> {
    let ast_sigs = skc_corelib::rustlib_methods::provided_methods();
    ast_sigs
        .iter()
        .map(|(classname, ast_sig)| make_rustlib_method_sig(classname, ast_sig, type_index))
        .collect()
}

// Create a SkMethod by converting ast_sig to hir_sig
fn make_rustlib_method_sig(
    classname: &ClassFullname,
    ast_sig: &AstMethodSignature,
    type_index: &TypeIndex,
) -> MethodSignature {
    let class_typarams = type_index
        .get(&classname.to_type_fullname())
        .unwrap_or_else(|| panic!("no such built-in class: {}", classname));
    make_hir_sig(classname, class_typarams, ast_sig)
}

// Convert ast_sig into hir_sig
fn make_hir_sig(
    type_name: &ClassFullname,
    class_typarams: &[ty::TyParam],
    ast_sig: &AstMethodSignature,
) -> MethodSignature {
    let class_typaram_names = class_typarams.iter().map(|x| &x.name).collect::<Vec<_>>();
    let fullname = method_fullname(type_name.clone().into(), &ast_sig.name.0);
    let ret_ty = if let Some(typ) = &ast_sig.ret_typ {
        convert_typ(typ, &class_typaram_names)
    } else {
        ty::raw("Void")
    };
    let params = convert_params(&ast_sig.params, &class_typaram_names);
    MethodSignature {
        fullname,
        ret_ty,
        params,
        // TODO: Fix this when a rustlib method has method typaram
        typarams: Default::default(),
        asyncness: Asyncness::Unknown,
        is_virtual: false,
    }
}

// Make hir params from ast params
fn convert_params(params: &[shiika_ast::Param], class_typarams: &[&String]) -> Vec<MethodParam> {
    params
        .iter()
        .map(|x| convert_param(x, class_typarams))
        .collect()
}

// Make hir param from ast param
fn convert_param(param: &shiika_ast::Param, class_typarams: &[&String]) -> MethodParam {
    MethodParam {
        name: param.name.to_string(),
        ty: convert_typ(&param.typ, class_typarams),
        has_default: param.default_expr.is_some(),
    }
}

// Make TermTy from UnresolvedTypeName
fn convert_typ(typ: &UnresolvedTypeName, class_typarams: &[&String]) -> TermTy {
    if typ.args.is_empty() {
        let s = typ.names.join("::");
        if let Some(i) = class_typarams.iter().position(|name| **name == s) {
            ty::typaram_ref(s, ty::TyParamKind::Class, i).into_term_ty()
        } else {
            ty::raw(&typ.names.join("::"))
        }
    } else {
        let type_args = typ
            .args
            .iter()
            .map(|n| convert_typ(n, class_typarams))
            .collect();
        ty::spe(&typ.names.join("::"), type_args)
    }
}
