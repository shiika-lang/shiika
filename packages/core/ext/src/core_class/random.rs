use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use shiika_ffi::core_class::{SkClass, SkFloat, SkInt, SkRandom};
use shiika_ffi_macro::shiika_method;

/// Called from `Random.new` and initializes the internal rng field.
#[shiika_method("Random#_initialize_rustlib")]
pub extern "C" fn random_initialize_rustlib(receiver: SkRandom, seed: SkInt) {
    let rng: StdRng = StdRng::seed_from_u64(seed.val() as u64);
    unsafe {
        *receiver.rng_ptr() = Box::into_raw(Box::new(rng)) as *mut u8;
    }
}

/// Create an instance of `Random` without explicit seed.
#[shiika_method("Meta:Random#_without_seed")]
pub extern "C" fn meta_random_without_seed(receiver: SkClass) -> SkRandom {
    let rnd = SkRandom::allocate(receiver.vtable(), receiver.0 as *const u8);
    let rng: StdRng = rand::make_rng();
    unsafe {
        *rnd.rng_ptr() = Box::into_raw(Box::new(rng)) as *mut u8;
    }
    rnd
}

/// Returns a random integer (end-exclusive).
#[shiika_method("Random#int")]
pub extern "C" fn random_int(receiver: SkRandom, from: SkInt, to: SkInt) -> SkInt {
    let rng = unsafe { &mut *(*receiver.rng_ptr() as *mut StdRng) };
    let f: i64 = from.val();
    let t: i64 = to.val();
    rng.random_range(f..t).into()
}

/// Returns a random float between 0.0 and 1.0 (end-exclusive).
#[shiika_method("Random#float")]
pub extern "C" fn random_float(receiver: SkRandom) -> SkFloat {
    let rng = unsafe { &mut *(*receiver.rng_ptr() as *mut StdRng) };
    rng.random::<f64>().into()
}
