
use crate::codegen::{CodeGen};
use skc_mir;

pub fn define( gen: &mut CodeGen,

    sk_types: &SkTypes, imports: &LibraryExports
               ) {
    let vtalbes = skc_mir::VTables::build(sk_types, imports);

}
