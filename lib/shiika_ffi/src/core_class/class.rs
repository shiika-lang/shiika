mod witness_table;
use crate::core_class::SkString;
pub use witness_table::WitnessTable;

#[repr(C)]
#[derive(Debug)]
pub struct SkClass(pub *mut ShiikaClass);

impl SkClass {
    pub fn new(ptr: *mut ShiikaClass) -> SkClass {
        SkClass(ptr)
    }

    pub fn dup(&self) -> SkClass {
        SkClass(self.0)
    }

    pub fn name(&self) -> &SkString {
        unsafe { &(*self.0).name }
    }

    pub fn witness_table(&self) -> &WitnessTable {
        unsafe {
            (*self.0).witness_table.as_ref().unwrap_or_else(|| {
                panic!(
                    "[BUG] witness_table is null: {:?}, {}",
                    self,
                    self.name().as_str()
                )
            })
        }
    }

    pub fn witness_table_mut(&mut self) -> &mut WitnessTable {
        unsafe {
            (*self.0).witness_table.as_mut().unwrap_or_else(|| {
                panic!(
                    "[BUG] witness_table is null: {:?}, {}",
                    self,
                    self.name().as_str()
                )
            })
        }
    }

    pub fn ensure_witness_table(&mut self) {
        unsafe {
            if (*self.0).witness_table.is_null() {
                (*self.0).witness_table = Box::into_raw(Box::new(WitnessTable::new()));
            }
        }
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
