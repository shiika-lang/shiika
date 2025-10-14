mod witness_table;
use crate::core_class::SkString;
pub use witness_table::WitnessTable;

#[repr(C)]
#[derive(Debug)]
pub struct SkClass(*mut ShiikaClass);

impl SkClass {
    pub fn new(ptr: *mut ShiikaClass) -> SkClass {
        SkClass(ptr)
    }

    pub fn dup(&self) -> SkClass {
        SkClass(self.0)
    }

    pub fn witness_table(&self) -> &WitnessTable {
        unsafe { (*self.0).witness_table.as_ref().unwrap() }
    }

    pub fn witness_table_mut(&mut self) -> &mut WitnessTable {
        unsafe { (*self.0).witness_table.as_mut().unwrap() }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaClass {
    vtable: *const u8,
    metaclass_obj: SkClass,
    name: SkString,
    witness_table: *mut WitnessTable,
}
