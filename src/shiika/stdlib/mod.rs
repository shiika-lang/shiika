mod float;
use crate::shiika::hir::*;

pub fn stdlib_methods() -> Vec<SkMethod> {
    float::stdlib_methods()
}
