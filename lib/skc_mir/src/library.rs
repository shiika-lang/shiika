use crate::{Mir, VTables};
use serde::{Deserialize, Serialize};
use shiika_core::{names::ConstFullname, ty::TermTy};
use skc_hir::SkModules;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LibraryExports {
    pub sk_modules: SkModules,
    pub vtables: VTables,
    pub constants: HashMap<ConstFullname, TermTy>,
}

impl LibraryExports {
    pub fn new(mir: &Mir) -> LibraryExports {
        LibraryExports {
            // PERF: how to generate json without cloning?
            sk_modules: mir.hir.sk_modules.clone(),
            vtables: mir.vtables.clone(),
            constants: mir.hir.constants.clone(),
        }
    }
}
