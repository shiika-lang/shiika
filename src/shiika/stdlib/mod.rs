mod float;
use crate::shiika::ty::*;
use crate::shiika::hir::*;

pub fn define_method(fullname: &str, arg_tys: Vec<TermTy>, ret_ty: TermTy, gen: GenMethodBody) -> SkMethod {
    SkMethod {
        fullname: fullname.to_string(),
        signature: MethodSignature {
            ret_ty: ret_ty,
            arg_tys: arg_tys,
        },
        body: SkMethodBody::RustMethodBody{ gen: gen }
    }
}

pub fn stdlib_methods() -> Vec<SkMethod> {
    let mut v = vec!();
    float::define_methods(&mut v);
    v
}
