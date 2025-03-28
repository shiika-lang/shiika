//! Define `new` of each class.
//! (implementation of `new` is decided from the signature
//! of `initialize`)
use crate::hir::expr::untyped;
use crate::hir::{self, Param};
use crate::names::FunctionName;
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

        let (new, new_sig) = create_new(class_dict, &lit_ty);
        prog.methods.push(new);

        adds.push((sk_class.fullname().clone(), new_sig.clone()));
    }
    for (class_name, new_sig) in adds {
        class_dict.add_method(&class_name, new_sig);
    }
}

fn create_new(class_dict: &ClassDict, meta_ty: &LitTy) -> (hir::Method<()>, MethodSignature) {
    let instance_ty = meta_ty.instance_ty().to_term_ty();
    let initialize = find_initialize(class_dict, &instance_ty);
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
    let args = initialize
        .sig
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| untyped(hir::Expr::ArgRef(i, param.name.clone())))
        .collect();
    exprs.push(untyped(hir::Expr::MethodCall(
        Box::new(untyped(hir::Expr::LVarRef(tmp_name.to_string()))),
        method_firstname("initialize"),
        args,
    )));

    // - Return it
    exprs.push(untyped(hir::Expr::Return(Box::new(untyped(
        hir::Expr::LVarRef(tmp_name.to_string()),
    )))));

    let m = hir::Method {
        name: FunctionName::method(meta_ty.base_name.clone(), "new"),
        params: initialize
            .sig
            .params
            .clone()
            .iter()
            .map(|param| Param {
                ty: param.ty.clone(),
                name: param.name.clone(),
            })
            .collect(),
        ret_ty: instance_ty.clone(),
        body_stmts: untyped(hir::Expr::Exprs(exprs)),
        self_ty: Some(instance_ty.meta_ty()),
    };
    let sig = MethodSignature {
        fullname: method_fullname(meta_ty.to_term_ty().base_type_name(), "new"),
        ret_ty: instance_ty.clone(),
        params: initialize.sig.params.clone(),
        typarams: initialize.sig.typarams.clone(),
    };
    (m, sig)
}

/// Returns the `initialize` method of the class (if none, its ancestor's)
fn find_initialize(class_dict: &ClassDict, class: &TermTy) -> FoundMethod {
    class_dict
        .lookup_method(
            class,
            &method_firstname("initialize"),
            &LocationSpan::internal(),
        )
        .unwrap()
}
