mod witness_table;
use crate::core_class::SkString;
use std::collections::HashMap;
pub use witness_table::WitnessTable;

#[repr(C)]
#[derive(Debug)]
pub struct SkClass(pub *mut ShiikaClass);

unsafe impl Send for SkClass {}

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

    pub fn vtable(&self) -> *const u8 {
        unsafe { (*self.0).vtable }
    }

    pub fn type_args(&self) -> &Vec<SkClass> {
        unsafe { &*(*self.0).type_args }
    }

    pub fn metaclass_obj(&self) -> SkClass {
        let metaclass_obj = unsafe { &(*self.0).metaclass_obj };
        SkClass::new(metaclass_obj.0)
    }

    //    pub fn specialize(self, tyargs: Vec<SkClass>) -> SkClass {
    //        class_specialize(self, tyargs)
    //    }

    pub fn specialized_classes(&mut self) -> &mut HashMap<String, *mut ShiikaClass> {
        unsafe { (*self.0).specialized_classes.as_mut().unwrap() }
    }

    pub fn erasure_class(&self) -> SkClass {
        let erasure_cls = unsafe { &(*self.0).erasure_cls };
        if erasure_cls.0.is_null() {
            self.dup()
        } else {
            erasure_cls.dup()
        }
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

// TODO: Remove `pub` from fields
#[repr(C)]
#[derive(Debug)]
pub struct ShiikaClass {
    pub vtable: *const u8,
    pub metaclass_obj: SkClass,
    pub name: SkString,
    pub specialized_classes: *mut HashMap<String, *mut ShiikaClass>,
    pub type_args: *mut Vec<SkClass>,
    pub witness_table: *mut WitnessTable,
    // `Array<Int>` -> `Array`
    // `Pair<Int, Bool>` -> `Pair`
    // `Object` -> null (means that its erasure is itself)
    pub erasure_cls: SkClass,
}
