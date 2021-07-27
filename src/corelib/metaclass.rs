use crate::hir::*;
use crate::ty;
use std::collections::HashMap;

pub fn ivars() -> HashMap<String, SkIVar> {
    let mut ivars = HashMap::new();
    ivars.insert(
        "@base_name".to_string(),
        SkIVar {
            name: "@base_name".to_string(),
            idx: 0,
            ty: ty::raw("String"),
            readonly: true,
        },
    );
    ivars
}
