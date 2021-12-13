//! The class `Metaclass`.
//! Instances of this class are metaclass objects (rarely appears on a program though.)
use crate::class;
use shiika_core::ty;
use skc_hir::SkIVar;
use std::collections::HashMap;

pub const IVAR_BASE_NAME_IDX: usize = class::N_IVARS;

pub fn ivars() -> HashMap<String, SkIVar> {
    // Inherit ivars of `Class`
    let mut ivars = class::ivars();
    ivars.insert(
        "@base_name".to_string(),
        SkIVar {
            name: "@base_name".to_string(),
            idx: IVAR_BASE_NAME_IDX,
            ty: ty::raw("String"),
            readonly: true,
        },
    );
    ivars
}
