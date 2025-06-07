use crate::{Mir, VTables};
use serde::{Deserialize, Serialize};
use shiika_core::{names::ConstFullname, ty::TermTy};
use skc_hir::SkTypes;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub struct LibraryExports {
    pub sk_types: SkTypes,
    pub vtables: VTables,
    // TODO: This should be Vec because initialize order matters
    pub constants: HashMap<ConstFullname, TermTy>,
}

impl LibraryExports {
    pub fn empty() -> LibraryExports {
        LibraryExports {
            sk_types: SkTypes::default(),
            vtables: VTables::default(),
            constants: HashMap::new(),
        }
    }

    pub fn new(mir: &Mir) -> LibraryExports {
        LibraryExports {
            // PERF: how to generate json without cloning?
            sk_types: mir.hir.sk_types.clone(),
            vtables: mir.vtables.clone(),
            constants: mir.hir.constants.clone(),
        }
    }
}
