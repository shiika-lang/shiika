#[no_mangle]
pub extern "C" fn print(n: i64) {
    println!("{}", n);
}
