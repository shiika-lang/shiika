pub mod vtable;
use crate::hir::{Hir, SkClass};
pub use crate::mir::vtable::VTables;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Mir {
    pub hir: Hir,
    pub vtables: VTables,
    pub imported_classes: Vec<SkClass>,
}

pub fn build(orig_hir: Hir) -> Mir {
    let vtables = VTables::build(&orig_hir.sk_classes);
    let (hir, imported_classes) = extract_imported_classes(orig_hir);
    Mir {
        hir,
        vtables,
        imported_classes,
    }
}

/// Remove imported classes from hir.sk_classes
fn extract_imported_classes(mut hir: Hir) -> (Hir, Vec<SkClass>) {
    let mut sk_classes = HashMap::new();
    let mut imported_classes = vec![];
    for (name, class) in hir.sk_classes {
        if class.foreign {
            imported_classes.push(class);
        } else {
            sk_classes.insert(name, class);
        }
    }
    hir.sk_classes = sk_classes;
    (hir, imported_classes)
}
