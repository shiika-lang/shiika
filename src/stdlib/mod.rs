use std::collections::HashMap;
mod float;
mod int;
mod object;
use crate::ty::*;
use crate::hir::*;

pub fn create_classes() -> Vec<SkClass> {
    vec![
        float::create_class(),
        int::create_class(),
        object::create_class(),
    ]
}

pub fn define_method(hash: &mut HashMap<String, SkMethod>, class_name: &str, name: &str, arg_tys: Vec<TermTy>, ret_ty: TermTy, gen: GenMethodBody) {
    let method = SkMethod {
        signature: MethodSignature {
            name: name.to_string(),
            fullname: (class_name.to_string() + "#" + name),
            ret_ty: ret_ty,
            arg_tys: arg_tys,
        },
        body: Some(SkMethodBody::RustMethodBody{ gen: gen })
    };
    hash.insert(name.to_string(), method);
}
