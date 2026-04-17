#[repr(C)]
#[derive(Debug)]
pub struct SkRandom(*mut ShiikaRandom);

unsafe impl Send for SkRandom {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaRandom {
    vtable: *const u8,
    class_obj: *const u8,
    rng: *mut u8, // Actually *mut StdRng, but opaque here
}

impl SkRandom {
    /// Get a mutable reference to the rng pointer field.
    pub fn rng_ptr(&self) -> *mut *mut u8 {
        unsafe { &mut (*self.0).rng }
    }

    /// Allocate a new SkRandom with the given vtable and class object.
    pub fn allocate(vtable: *const u8, class_obj: *const u8) -> SkRandom {
        let inner = Box::new(ShiikaRandom {
            vtable,
            class_obj,
            rng: std::ptr::null_mut(),
        });
        SkRandom(Box::into_raw(inner))
    }
}
