mod float;
mod int;
mod object;
mod void;
use crate::ty::*;
use crate::hir::*;

pub fn create_classes() -> Vec<SkClass> {
    vec![
        float::create_class(),
        int::create_class(),
        object::create_class(),
        void::create_class(),
    ]
}

pub fn create_method(class_name: &str,
                     name: &str,
                     param_tys: Vec<TermTy>,
                     ret_ty: TermTy,
                     gen: GenMethodBody) -> SkMethod {
    SkMethod {
        signature: MethodSignature {
            name: name.to_string(),
            fullname: (class_name.to_string() + "#" + name),
            ret_ty: ret_ty,
            params: param_tys.into_iter().map(|ty| MethodParam { name: "".to_string(), ty: ty }).collect(),
        },
        body: Some(SkMethodBody::RustMethodBody{ gen: gen })
    }
}
