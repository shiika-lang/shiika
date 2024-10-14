use shiika_ffi::core_class::SkInt;

#[no_mangle]
pub extern "C" fn print(n: SkInt) {
    println!("{}", n.val());
}
