mod alloc;
mod sk_obj;

use sk_obj::*;
use std::io::Write;

#[no_mangle]
pub extern "C" fn shiika_puts(sk_str: *const i8) {
    unsafe {
        let s = sk_str as *const SkString;
        let _result = std::io::stdout().write_all((*s).as_slice());
        println!("");
    }
}
