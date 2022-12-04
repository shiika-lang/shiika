use crate::builtin::{SkClass, SkStr};
use shiika_ffi_macro::shiika_method;
use std::fs;

#[shiika_method("Meta:File#read")]
pub extern "C" fn meta_file_read(_receiver: SkClass, path: SkStr) -> SkStr {
    // TODO: Support reading binary (i.e. non-utf8) files
    let content = fs::read_to_string(path.as_str()).unwrap(); // TODO: Return SkResult
    content.into()
}

#[shiika_method("Meta:File#write")]
pub extern "C" fn meta_file_write(_receiver: SkClass, path: SkStr, content: SkStr) {
    fs::write(path.as_str(), content.as_byteslice()).unwrap(); // TODO: Return SkResult
}
