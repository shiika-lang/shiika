//! Define `new` of each class.
//! (implementation of `new` is decided from the signature
//! of `initialize`)
use crate::hir::expr::untyped;
use crate::hir::{self, Param};
use shiika_ast::LocationSpan;
use shiika_core::names::{method_firstname, method_fullname};
use shiika_core::ty::{LitTy, TermTy};
use skc_ast2hir::class_dict::{ClassDict, FoundMethod};
use skc_hir::MethodSignature;

pub fn run(prog: &mut hir::Program<()>, class_dict: &mut ClassDict) {
    let mut adds = vec![];
    for sk_class in class_dict.sk_types.sk_classes() {
        let lit_ty = sk_class.lit_ty();
        if !sk_class.lit_ty().is_meta {
            continue;
        }

        let new = create_new(class_dict, &lit_ty);
        adds.push((sk_class.fullname().clone(), new.sig.clone()));
        prog.methods.push(new);
    }
    for (class_name, new_sig) in adds {
        class_dict.add_method(&class_name, new_sig);
    }
}

fn create_new(class_dict: &ClassDict, meta_ty: &LitTy) -> hir::Method<()> {
    let instance_ty = meta_ty.instance_ty().to_term_ty();
    let opt_initialize = find_initialize(class_dict, &instance_ty);
    let tmp_name = "tmp";
    let mut exprs = vec![];

    // - Allocate memory and set .class (which is the receiver of .new)
    exprs.push(untyped(hir::Expr::LVarDecl(
        tmp_name.to_string(),
        Box::new(untyped(hir::Expr::CreateObject(
            instance_ty.base_class_name(),
        ))),
    )));

    // - Call initialize on it
    let mut params = vec![];
    let mut typarams = vec![];
    if let Some(initialize) = opt_initialize {
        let args = initialize
            .sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| untyped(hir::Expr::ArgRef(i, param.name.clone())))
            .collect();
        exprs.push(untyped(hir::Expr::ResolvedMethodCall(
            hir::expr::MethodCallType::Direct,
            Box::new(untyped(hir::Expr::LVarRef(tmp_name.to_string()))),
            method_firstname("initialize"),
            args,
        )));
        params = initialize.sig.params.clone();
        typarams = initialize.sig.typarams.clone();
    }

    // - Return it
    exprs.push(untyped(hir::Expr::Return(Box::new(untyped(
        hir::Expr::LVarRef(tmp_name.to_string()),
    )))));

    let method_name = method_fullname(meta_ty.to_term_ty().fullname.clone(), "new");
    let sig = MethodSignature {
        fullname: method_name.clone(),
        ret_ty: instance_ty.clone(),
        params: params.clone(),
        typarams,
        asyncness: skc_hir::Asyncness::Unknown,
    };
    hir::Method {
        name: method_name.clone().into(),
        sig,
        params: params
            .iter()
            .map(|param| Param {
                ty: param.ty.clone(),
                name: param.name.clone(),
            })
            .collect(),
        ret_ty: instance_ty.clone(),
        body_stmts: untyped(hir::Expr::Exprs(exprs)),
        self_ty: instance_ty.meta_ty(),
    }
}

/// Returns the `initialize` method of the class (if none, its ancestor's)
fn find_initialize(class_dict: &ClassDict, class: &TermTy) -> Option<FoundMethod> {
    class_dict
        .lookup_method(
            class,
            &method_firstname("initialize"),
            &LocationSpan::internal(),
        )
        .ok()
}
