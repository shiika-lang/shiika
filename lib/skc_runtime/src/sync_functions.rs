use shiika_ffi::core_class::SkInt;
use shiika_ffi_macro::shiika_method;

#[shiika_method("print")]
pub extern "C" fn print(n: SkInt) {
    println!("{}", n.val());
}
