use crate::hir::*;
use crate::ty;
use std::collections::HashMap;

pub fn ivars() -> HashMap<String, SkIVar> {
    let mut ivars = HashMap::new();
    ivars.insert(
        "@name".to_string(),
        SkIVar {
            name: "@name".to_string(),
            idx: 0,
            ty: ty::raw("String"),
            readonly: true,
        },
    );
    ivars.insert(
        "@vtable".to_string(),
        SkIVar {
            name: "@vtable".to_string(),
            idx: 1,
            ty: ty::raw("Shiika::Internal::Ptr"),
            readonly: true,
        },
    );
    ivars
}
