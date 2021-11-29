mod library;
mod vtable;
pub use crate::library::LibraryExports;
pub use crate::vtable::{VTable, VTables};
use skc_hir::Hir;

#[derive(Debug)]
pub struct Mir {
    pub hir: Hir,
    pub vtables: VTables,
    pub imports: LibraryExports,
}

pub fn build(hir: Hir, imports: LibraryExports) -> Mir {
    let vtables = VTables::build(&hir.sk_classes, &imports);
    Mir {
        hir,
        vtables,
        imports,
    }
}