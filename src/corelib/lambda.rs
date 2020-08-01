use std::collections::HashMap;
use crate::corelib::*;
use crate::hir::*;
use crate::ty;

pub fn create_methods_1() -> Vec<SkMethod> {
    vec![
        create_method_generic(
            "Fn1",
            "call(arg1: A) -> Z",
            |code_gen, function| {
                //    let receiver = function.get_params()[0];
                code_gen.builder.build_return(None);
                Ok(())
            },
            &vec!["A".to_string(), "Z".to_string()]
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

