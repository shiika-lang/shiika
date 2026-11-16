//! Make every thread of the process known to bdwgc from its very start.
//!
//! bdwgc is the Rust global allocator (see `allocator.rs`), so the data
//! `std::thread::spawn` hands to a new thread (`Box<ThreadInit>`) is a GC
//! object. std passes it only as the argument of `pthread_create`, where the
//! collector cannot see it, and the new thread is invisible to the collector
//! until it registers itself. If a collection runs in between, the start data
//! is reclaimed and the new thread crashes in `thread_start` (SIGSEGV, or
//! "Duplicate large block deallocation").
//!
//! bdwgc's answer is to create threads with `GC_pthread_create`, which keeps
//! the argument reachable and does not return until the child is registered.
//! In C that is done by a macro in `gc.h`; Rust's std and tokio call
//! `pthread_create` directly. So we define `pthread_create` (and `join`/
//! `detach`, which bdwgc needs to maintain its thread table) in the executable
//! and forward them to the `GC_` versions. Since this library is linked
//! statically, the calls from std resolve to these definitions.
use libc::{c_int, c_void, pthread_attr_t, pthread_t};
use std::cell::Cell;
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

type StartRoutine = extern "C" fn(*mut c_void) -> *mut c_void;
type CreateFn =
    unsafe extern "C" fn(*mut pthread_t, *const pthread_attr_t, StartRoutine, *mut c_void) -> c_int;
type JoinFn = unsafe extern "C" fn(pthread_t, *mut *mut c_void) -> c_int;
type DetachFn = unsafe extern "C" fn(pthread_t) -> c_int;

extern "C" {
    fn GC_pthread_create(
        thread: *mut pthread_t,
        attr: *const pthread_attr_t,
        start: StartRoutine,
        arg: *mut c_void,
    ) -> c_int;
    fn GC_pthread_join(thread: pthread_t, retval: *mut *mut c_void) -> c_int;
    fn GC_pthread_detach(thread: pthread_t) -> c_int;
}

/// False until `enable()` is called. Before that, the functions below just
/// call the real ones: the collector's own marker threads are created while
/// it initializes and must not be registered as mutator threads.
static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// True while this thread is inside a `GC_pthread_*` function. Those call
    /// the plain `pthread_*` names internally, which resolve back to the
    /// definitions in this file.
    static IN_GC: Cell<bool> = const { Cell::new(false) };
}

/// Start routing thread creation through bdwgc. Must be called after
/// `GC_allow_register_threads()` (which starts the marker threads) and before
/// any other thread is created.
pub fn enable() {
    ENABLED.store(true, Ordering::SeqCst);
}

fn should_forward() -> bool {
    ENABLED.load(Ordering::Relaxed) && !IN_GC.with(|f| f.get())
}

fn with_in_gc<T>(f: impl FnOnce() -> T) -> T {
    IN_GC.with(|flag| flag.set(true));
    let result = f();
    IN_GC.with(|flag| flag.set(false));
    result
}

/// Address of the next definition of `name` after this executable (i.e. the
/// one in libc).
fn real(cache: &AtomicUsize, name: &CStr) -> usize {
    let mut addr = cache.load(Ordering::Relaxed);
    if addr == 0 {
        addr = unsafe { libc::dlsym(libc::RTLD_NEXT, name.as_ptr()) } as usize;
        assert!(addr != 0, "{:?} not found", name);
        cache.store(addr, Ordering::Relaxed);
    }
    addr
}

#[no_mangle]
pub unsafe extern "C" fn pthread_create(
    thread: *mut pthread_t,
    attr: *const pthread_attr_t,
    start: StartRoutine,
    arg: *mut c_void,
) -> c_int {
    static REAL: AtomicUsize = AtomicUsize::new(0);
    if should_forward() {
        with_in_gc(|| GC_pthread_create(thread, attr, start, arg))
    } else {
        let f: CreateFn = std::mem::transmute(real(&REAL, c"pthread_create"));
        f(thread, attr, start, arg)
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_join(thread: pthread_t, retval: *mut *mut c_void) -> c_int {
    static REAL: AtomicUsize = AtomicUsize::new(0);
    if should_forward() {
        with_in_gc(|| GC_pthread_join(thread, retval))
    } else {
        let f: JoinFn = std::mem::transmute(real(&REAL, c"pthread_join"));
        f(thread, retval)
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_detach(thread: pthread_t) -> c_int {
    static REAL: AtomicUsize = AtomicUsize::new(0);
    if should_forward() {
        with_in_gc(|| GC_pthread_detach(thread))
    } else {
        let f: DetachFn = std::mem::transmute(real(&REAL, c"pthread_detach"));
        f(thread)
    }
}
