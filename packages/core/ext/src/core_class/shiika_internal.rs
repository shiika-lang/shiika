use shiika_ffi::core_class::{SkInt, SkObject};
use shiika_ffi_macro::shiika_method;

#[shiika_method("Meta:Shiika::Internal#p")]
pub extern "C" fn meta_shiika_internal_p(_receiver: SkObject, value: *const u64, len: SkInt) {
    unsafe {
        let n = len.val() as usize;
        for i in 0..n {
            let v = value.add(i);
            println!("0x{:x}: {:x} ", v as u64, *v);
        }
    }
}
