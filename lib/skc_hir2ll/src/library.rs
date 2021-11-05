use crate::hir::*;
use crate::mir::*;
use serde::{Deserialize, Serialize};
use shiika_core::{names::*, ty::*};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LibraryExports {
    pub sk_classes: SkClasses,
    pub vtables: VTables,
    pub constants: HashMap<ConstFullname, TermTy>,
}

impl LibraryExports {
    pub fn new(mir: &Mir) -> LibraryExports {
        LibraryExports {
            // PERF: how to generate json without cloning?
            sk_classes: mir.hir.sk_classes.clone(),
            vtables: mir.vtables.clone(),
            constants: mir.hir.constants.clone(),
        }
    }
}
