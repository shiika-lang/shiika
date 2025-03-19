//! Define `new` of each class.
//! (implementation of `new` is decided from the signature
//! of `initialize`)
use crate::hir::expr::untyped;
use crate::hir::{self, Param};
use crate::names::FunctionName;
use shiika_ast::LocationSpan;
use shiika_core::names::{method_firstname, method_fullname, ClassFullname};
use shiika_core::ty::TermTy;
use skc_ast2hir::class_dict::{ClassDict, FoundMethod};
use skc_hir::MethodSignature;

pub fn run(prog: &mut hir::Program<()>, class_dict: &mut ClassDict) {
    let mut adds = vec![];
    for sk_class in class_dict.sk_types.sk_classes() {
        if sk_class.base.fullname().0 == "Never" {
            continue;
        }

        let (new, new_sig) = create_new(class_dict, &sk_class.fullname());
        prog.methods.push(new);

        adds.push((sk_class.fullname().clone(), new_sig.clone()));
    }
    for (class_name, new_sig) in adds {
        class_dict.add_method(&class_name, new_sig);
    }
}

fn create_new(class_dict: &ClassDict, class: &ClassFullname) -> (hir::Method<()>, MethodSignature) {
    let instance_ty = class.to_ty();
    let initialize = find_initialize(class_dict, &instance_ty);
    let tmp_name = "tmp";
    let mut exprs = vec![];
    exprs.push(untyped(hir::Expr::Alloc(tmp_name.to_string())));

    // - Allocate memory and set .class (which is the receiver of .new)
    exprs.push(untyped(hir::Expr::Assign(
        tmp_name.to_string(),
        Box::new(untyped(hir::Expr::CreateObject(class.clone()))),
    )));

    // - Call initialize on it
    let args = initialize
        .sig
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| untyped(hir::Expr::ArgRef(i, param.name.clone())))
        .collect();
    exprs.push(untyped(hir::Expr::FunCall(
        Box::new(untyped(hir::Expr::FuncRef(initialize.sig.clone().into()))),
        args,
    )));

    // - Return it
    exprs.push(untyped(hir::Expr::Return(Box::new(untyped(
        hir::Expr::LVarRef(tmp_name.to_string()),
    )))));

    let m = hir::Method {
        name: FunctionName::method(class.0.clone(), "new"),
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
        fullname: method_fullname(class.meta_name().to_type_fullname(), "new"),
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
