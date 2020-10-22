pub mod vtable;
pub use crate::mir::vtable::VTables;
use crate::hir::Hir;

#[derive(Debug)]
pub struct Mir {
    pub hir: Hir,
    pub vtables: VTables,
}

pub fn build(hir: Hir) -> Mir {
    let vtables = vtable::build_vtables(&hir.sk_classes);
    Mir { hir, vtables }
}
