use shiika_ffi::core_class::SkInt;
use shiika_ffi_macro::shiika_method;

#[shiika_method("Object#print")]
pub extern "C" fn print(_receiver: SkInt, n: SkInt) {
    println!("{}", n.val());
}
