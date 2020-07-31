use std::collections::HashMap;
use crate::corelib::create_method;
use crate::hir::*;
use crate::ty;

pub fn create_methods_1() -> Vec<SkMethod> {
    vec![
        create_method(
            "Fn1",
            "call(T) -> T",
            |code_gen, function| {
            }
        )
    ]
}

pub fn ivars() -> HashMap<String, SkIVar> {
    let mut ivars = HashMap::new();
    ivars.insert(
        "@func".to_string(),
        SkIVar {
            name: "@func".to_string(),
            idx: 0,
            ty: ty::raw("Shiika::Internal::Ptr"),
            readonly: true,
        },
    );
    ivars.insert(
        "@freevars".to_string(),
        SkIVar {
            name: "@freevars".to_string(),
            idx: 1,
            ty: ty::spe("Array", vec![ty::raw("Shiika::Internal::Ptr")]),
            readonly: true,
        },
    );
    ivars
}

