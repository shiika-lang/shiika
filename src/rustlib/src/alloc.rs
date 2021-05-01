use bdwgc_alloc::Allocator;
use std::alloc::Layout;
use std::os::raw::c_void;

#[global_allocator]
static GLOBAL_ALLOCATOR: Allocator = Allocator;

const DEFAULT_ALIGNMENT: usize = 8;

#[no_mangle]
pub extern "C" fn shiika_malloc(size: usize) -> *mut c_void {
    (unsafe { std::alloc::alloc(Layout::from_size_align(size, DEFAULT_ALIGNMENT).unwrap()) })
        as *mut c_void
}

#[no_mangle]
pub extern "C" fn shiika_realloc(pointer: *mut c_void, size: usize) -> *mut c_void {
    // Layouts are ignored by the bdwgc global allocator.
    (unsafe {
        std::alloc::realloc(
            pointer as *mut u8,
            Layout::from_size_align(0, DEFAULT_ALIGNMENT).unwrap(),
            size,
        )
    }) as *mut c_void
}
