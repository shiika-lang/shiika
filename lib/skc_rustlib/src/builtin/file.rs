use crate::builtin::{SkClass, SkResult, SkStr};
use shiika_ffi_macro::shiika_method;
use std::fs;

#[shiika_method("Meta:File#read")]
pub extern "C" fn meta_file_read(_receiver: SkClass, path: SkStr) -> SkResult {
    // TODO: Support reading binary (i.e. non-utf8) files by using [u8]
    match fs::read_to_string(path.as_str()) {
        Ok(content) => SkResult::ok(SkStr::new(content)),
        Err(e) => SkResult::fail(format!("{}", e)),
    }
}

#[shiika_method("Meta:File#write")]
pub extern "C" fn meta_file_write(_receiver: SkClass, path: SkStr, content: SkStr) -> SkResult {
    fs::write(path.as_str(), content.as_byteslice()).into()
}
