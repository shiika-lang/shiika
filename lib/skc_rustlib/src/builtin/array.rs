use crate::builtin::object::ShiikaObject;
use crate::builtin::{SkInt, SkPtr};
use shiika_ffi_macro::shiika_method;

#[repr(C)]
#[derive(Debug)]
pub struct SkAry<T>(*mut ShiikaArray, [T; 0]);

#[repr(C)]
#[derive(Debug)]
struct ShiikaArray {
    vtable: *const u8,
    class_obj: *const u8,
    capa: SkInt,
    n_items: SkInt,
    items: SkPtr,
}

impl<T> SkAry<T> {
    //    /// Shallow clone
    //    pub fn dup(&self) -> SkAry<T> {
    //        SkAry::<T>(self.0, [])
    //    }

    /// Returns iterator
    pub fn iter(&self) -> SkAryIter<T> {
        SkAryIter {
            sk_ary: self,
            idx: 0,
        }
    }

    //    /// Create a `Vec` that has the same elements
    //    pub fn to_vec(&self) -> Vec<*mut T> {
    //        self.iter().collect()
    //    }

    /// Returns the number of elements
    pub fn len(&self) -> usize {
        unsafe { (*self.0).n_items.val() as usize }
    }

    /// Returns the element
    /// Panics if idx is too large
    pub fn get(&self, idx: usize) -> *mut T {
        if idx >= self.len() {
            panic!("idx too large (len: {}, idx: {})", self.len(), idx);
        }
        unsafe {
            let items_ptr = (*self.0).items.unbox() as *const *mut T;
            let item_ptr = items_ptr.offset(idx as isize);
            *item_ptr
        }
    }
}

#[shiika_method("Array#[]")]
pub extern "C" fn array_get(receiver: SkAry<ShiikaObject>, idx: SkInt) -> *const ShiikaObject {
    receiver.get(idx.val() as usize)
}

/// Iterates over each lvar scope.
pub struct SkAryIter<'ary, T> {
    sk_ary: &'ary SkAry<T>,
    idx: usize,
}

impl<'ary, T> Iterator for SkAryIter<'ary, T> {
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.sk_ary.len() {
            None
        } else {
            let item = self.sk_ary.get(self.idx);
            self.idx += 1;
            Some(item)
        }
    }
}
