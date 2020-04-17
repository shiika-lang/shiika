//use inkwell::values::*;
use std::collections::HashMap;
use crate::ty;
use crate::hir::*;
use crate::corelib::create_method;

pub fn create_methods() -> Vec<SkMethod> {
    vec![

    create_method("String", "to_s() -> String", |code_gen, function| {
        let this = function.get_params()[0];
        code_gen.builder.build_return(Some(&this));
        Ok(())
    }),

    ]
}

pub fn ivars() -> HashMap<String, SkIVar> {
    let mut ivars = HashMap::new();
    ivars.insert("@ptr".to_string(), SkIVar {
        name: "@ptr".to_string(),
        idx: 0,
        ty: ty::raw("Shiika::Internal::Ptr"),
        readonly: true,
    });
    ivars.insert("@bytesize".to_string(), SkIVar {
        name: "@bytesize".to_string(),
        idx: 1,
        ty: ty::raw("Int"),
        readonly: true,
    });
    ivars
}
