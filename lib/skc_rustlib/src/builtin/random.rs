use crate::builtin::{SkClass, SkFloat, SkInt};
use rand::prelude::{Rng, SeedableRng, StdRng};
use shiika_ffi_macro::shiika_method;
use shiika_ffi_macro::{shiika_const_ref, shiika_method_ref};

shiika_const_ref!("::Random", SkClass, "sk_Random");
shiika_method_ref!(
    "Meta:Random#new",
    fn(receiver: SkClass, seed: SkInt) -> SkRandom,
    "meta_random_new"
);

#[repr(C)]
#[derive(Debug)]
pub struct SkRandom(*mut ShiikaRandom);

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaRandom {
    vtable: *const u8,
    class_obj: *const u8,
    rng: *mut StdRng,
}

impl SkRandom {
    /// Returns the rng.
    fn rng(&mut self) -> &mut StdRng {
        unsafe { (*self.0).rng.as_mut().unwrap() }
    }
}

/// Called from `Random.new` and initializes internal fields.
#[shiika_method("Random#_initialize_rustlib")]
#[allow(non_snake_case)]
pub extern "C" fn random__initialize_rustlib(receiver: SkRandom, seed: SkInt) {
    let rng = SeedableRng::seed_from_u64(seed.into());
    unsafe {
        (*receiver.0).rng = Box::leak(Box::new(rng));
    }
}

/// Create an instance of `Random` without explicit seed.
#[shiika_method("Meta:Random#_without_seed")]
#[allow(non_snake_case)]
pub extern "C" fn meta_random__without_seed(_receiver: SkClass) -> SkRandom {
    let rnd = meta_random_new(sk_Random(), 0.into());
    // Replace the rng
    unsafe {
        (*rnd.0).rng = Box::leak(Box::new(SeedableRng::from_entropy()));
    }
    rnd
}

/// Returns a random float between 0.0 and 1.0 (end-exclusive).
#[shiika_method("Random#float")]
pub extern "C" fn random_float(mut receiver: SkRandom) -> SkFloat {
    receiver.rng().gen::<f64>().into()
}

/// Returns a random integer (end-exclusive).
#[shiika_method("Random#int")]
pub extern "C" fn random_int(mut receiver: SkRandom, from: SkInt, to: SkInt) -> SkInt {
    let f: i64 = from.into();
    let t: i64 = to.into();
    receiver.rng().gen_range(f..t).into()
}
