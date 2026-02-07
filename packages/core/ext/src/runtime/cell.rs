//! This module provides utilities to implement closures in Shiika.
use std::alloc::{alloc, Layout};

#[no_mangle]
pub extern "C" fn shiika_cell_new(value: u64) -> *mut u64 {
    unsafe {
        let layout = Layout::new::<u64>();
        let ptr = alloc(layout) as *mut u64;
        *ptr = value;
        ptr
    }
}

#[no_mangle]
pub extern "C" fn shiika_cell_get(cell: *mut u64) -> u64 {
    unsafe { *cell }
}

#[no_mangle]
pub extern "C" fn shiika_cell_set(cell: *mut u64, value: u64) {
    unsafe {
        *cell = value;
    }
}
