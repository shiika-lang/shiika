#![doc = include_str!("../README.md")]
#![no_std]

mod error;

use bdwgc_alloc_sys::{
    GC_SUCCESS, GC_alloc_lock, GC_alloc_unlock, GC_allow_register_threads, GC_gcollect,
    GC_get_stack_base, GC_init, GC_malloc, GC_realloc, GC_register_finalizer,
    GC_register_my_thread, GC_set_stackbottom, GC_stack_base, GC_unregister_my_thread,
};
use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
    ptr::null_mut,
};

/// An allocator.
pub struct Allocator;

impl Allocator {
    /// Locks a collector.
    pub fn lock() {
        unsafe { GC_alloc_lock() }
    }

    /// Unlocks a collector.
    pub fn unlock() {
        unsafe { GC_alloc_unlock() }
    }

    /// Initializes a collector.
    ///
    /// # Safety
    ///
    /// This function must be called in a main thread.
    pub unsafe fn initialize() {
        unsafe {
            GC_init();
            GC_allow_register_threads();
        }
    }

    /// Registers a current thread to a collector.
    ///
    /// # Safety
    ///
    /// This function must not be called in a main thread.
    pub unsafe fn register_current_thread() -> Result<(), error::Error> {
        let mut base = GC_stack_base {
            mem_base: null_mut(),
        };

        if unsafe { GC_get_stack_base(&mut base) } != GC_SUCCESS {
            return Err(error::Error::new("failed to get stack base"));
        } else if unsafe { GC_register_my_thread(&base) } != GC_SUCCESS {
            return Err(error::Error::new("failed to register a thread for GC"));
        }

        Ok(())
    }

    /// Sets a bottom of a stack.
    ///
    /// You do not have to call this function in most cases.
    /// A collector detects the bottom on initialization automatically.
    ///
    /// # Safety
    ///
    /// The bottom address must be valid.
    pub unsafe fn set_stack_bottom(bottom: *const u8) {
        unsafe {
            GC_set_stackbottom(
                null_mut(),
                &GC_stack_base {
                    mem_base: bottom.cast_mut().cast(),
                },
            )
        }
    }

    /// Unregisters a current thread from a collector.
    ///
    /// # Safety
    ///
    /// The thread must be registered already.
    pub unsafe fn unregister_current_thread() {
        unsafe { GC_unregister_my_thread() };
    }

    /// Runs a garbage collection forcibly.
    pub fn force_collect() {
        unsafe { GC_gcollect() }
    }

    /// Registers a finalizer of an object.
    ///
    /// # Safety
    ///
    /// The given finalizer must not be null and handle pointers properly.
    pub unsafe fn register_finalizer(
        ptr: *const c_void,
        finalizer: extern "C" fn(*mut c_void, *mut c_void),
        client_data: *const c_void,
    ) {
        unsafe {
            GC_register_finalizer(
                ptr.cast_mut(),
                Some(finalizer),
                client_data.cast_mut(),
                null_mut(),
                null_mut(),
            )
        };
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        (unsafe { GC_malloc(layout.size()) }) as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Experiment: never free explicitly; leave reclamation to the GC.
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, size: usize) -> *mut u8 {
        (unsafe { GC_realloc(ptr as *mut c_void, size) }) as *mut u8
    }
}
