use crate::{Mir, VTables};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shiika_core::{names::ConstFullname, ty::TermTy};
use skc_hir::SkTypes;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct LibraryExports {
    pub sk_types: SkTypes,
    pub vtables: VTables,
    pub constants: HashMap<ConstFullname, TermTy>,
}

impl LibraryExports {
    pub fn empty() -> LibraryExports {
        LibraryExports {
            sk_types: Default::default(),
            vtables: Default::default(),
            constants: Default::default(),
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

    pub fn merge(&mut self, other: LibraryExports) {
        self.sk_types.merge(other.sk_types);
        self.vtables.merge(other.vtables);
        self.constants.extend(other.constants);
    }

    pub fn save<P: AsRef<Path>>(&self, path_: P) -> Result<()> {
        let path = path_.as_ref();
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut f =
            fs::File::create(path).context(format!("failed to create {}", path.display()))?;
        f.write_all(json.as_bytes())
            .context(format!("failed to write {}", path.display()))?;
        Ok(())
    }
}
