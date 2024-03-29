//! The class `Class`.
//! Instances of this class are class objects.
use shiika_core::ty;
use skc_hir::SkIVar;
use std::collections::HashMap;

pub const N_IVARS: usize = 2;
pub const IVAR_NAME_IDX: usize = 0;

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
        "@specialized_classes".to_string(),
        SkIVar {
            name: "@specialized_classes".to_string(),
            idx: 1,
            ty: ty::raw("Object"),
            readonly: true,
        },
    );
    ivars.insert(
        "@type_args".to_string(),
        SkIVar {
            name: "@type_args".to_string(),
            idx: 2,
            ty: ty::raw("Object"),
            readonly: true,
        },
    );
    ivars.insert(
        "@witness_table".to_string(),
        SkIVar {
            name: "@witness_table".to_string(),
            idx: 3,
            ty: ty::raw("Object"),
            readonly: true,
        },
    );
    ivars
}
