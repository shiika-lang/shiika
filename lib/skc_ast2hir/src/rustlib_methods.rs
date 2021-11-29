use shiika_ast::AstMethodSignature;
use shiika_core::names::{ClassFullname, ConstName};
use shiika_core::{names::method_fullname, ty, ty::TermTy};
use skc_corelib::{self, Corelib};
use skc_hir::*;

/// Returns complete list of corelib classes/methods i.e. both those
/// implemented in Shiika and in Rust.
pub fn mix_with_corelib(corelib: Corelib) -> (SkClasses, SkMethods) {
    let rustlib_methods = make_rustlib_methods(&corelib);
    let mut sk_classes = corelib.sk_classes;
    let mut sk_methods = corelib.sk_methods;
    for (classname, m) in rustlib_methods.into_iter() {
        // Add to sk_classes
        let c = sk_classes
            .get_mut(&classname)
            .unwrap_or_else(|| panic!("not in sk_classes: {}", &classname));
        let first_name = &m.signature.fullname.first_name;
        debug_assert!(!c.method_sigs.contains_key(first_name));
        c.method_sigs
            .insert(first_name.clone(), m.signature.clone());
        // Add to sk_methods
        let v = sk_methods
            .get_mut(&classname)
            .unwrap_or_else(|| panic!("not in sk_methods: {}", &classname));
        v.push(m);
    }
    (sk_classes, sk_methods)
}

// Make SkMethod of corelib methods implemented in Rust
fn make_rustlib_methods(corelib: &Corelib) -> Vec<(ClassFullname, SkMethod)> {
    let sigs = skc_corelib::rustlib_methods::provided_methods();
    sigs.iter()
        .map(|(classname, ast_sig)| make_rustlib_method(classname, ast_sig, corelib))
        .collect()
}

// Create a SkMethod by converting ast_sig to hir_sig
fn make_rustlib_method(
    classname: &ClassFullname,
    ast_sig: &AstMethodSignature,
    corelib: &Corelib,
) -> (ClassFullname, SkMethod) {
    let class = corelib
        .sk_classes
        .get(classname)
        .unwrap_or_else(|| panic!("no such class in Corelib: {}", classname));
    let signature = make_hir_sig(class, ast_sig);
    let method = SkMethod {
        signature,
        body: SkMethodBody::RustLib,
        lvars: Default::default(),
    };
    (classname.clone(), method)
}

// Convert ast_sig into hir_sig
fn make_hir_sig(class: &SkClass, ast_sig: &AstMethodSignature) -> MethodSignature {
    let class_typarams = class.typarams.iter().map(|x| &x.name).collect::<Vec<_>>();
    let fullname = method_fullname(&class.fullname, &ast_sig.name.0);
    let ret_ty = if let Some(typ) = &ast_sig.ret_typ {
        convert_typ(typ, &class_typarams)
    } else {
        ty::raw("Void")
    };
    let params = convert_params(&ast_sig.params, &class_typarams);
    MethodSignature {
        fullname,
        ret_ty,
        params,
        // TODO: Fix this when a rustlib method has method typaram
        typarams: Default::default(),
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
    }
}

// Make TermTy from ConstName
fn convert_typ(typ: &ConstName, class_typarams: &[&String]) -> TermTy {
    if typ.args.is_empty() {
        let s = typ.names.join("::");
        if let Some(i) = class_typarams.iter().position(|name| **name == s) {
            ty::typaram(s, ty::TyParamKind::Class, i)
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
