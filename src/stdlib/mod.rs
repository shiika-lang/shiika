use std::collections::HashMap;
mod float;
mod int;
mod object;
use crate::shiika::ty::*;
use crate::shiika::hir::*;

pub fn create_classes() -> HashMap<String, SkClass> {
    let mut ret = HashMap::new();
    ret.insert("Float".to_string(), float::create_class());
    ret.insert("Int".to_string(), int::create_class());
    ret.insert("Object".to_string(), object::create_class());
    ret
}

pub fn define_method(hash: &mut HashMap<String, SkMethod>, class_name: &str, name: &str, arg_tys: Vec<TermTy>, ret_ty: TermTy, gen: GenMethodBody) {
    let method = SkMethod {
        id: MethodId(class_name.to_string() + "#" + name),
        signature: MethodSignature {
            ret_ty: ret_ty,
            arg_tys: arg_tys,
        },
        body: Some(SkMethodBody::RustMethodBody{ gen: gen })
    };
    hash.insert(name.to_string(), method);
}
