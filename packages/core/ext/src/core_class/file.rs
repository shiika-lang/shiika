use shiika_ffi::core_class::file::ShiikaFile;
use shiika_ffi::core_class::{SkClass, SkFile, SkInt, SkResult, SkString, SkVoid};
use shiika_ffi_macro::{shiika_method, shiika_method_ref};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::raw::c_void;
use std::ptr;

#[shiika_method("Meta:File#read")]
pub extern "C" fn meta_file_read(_receiver: SkClass, path: SkString) -> SkResult<SkString> {
    // TODO: Support reading binary (i.e. non-utf8) files by using [u8]
    _meta_file_read(path).into()
}

fn _meta_file_read(path: SkString) -> Result<SkString, std::io::Error> {
    Ok(SkString::from_rust_string(fs::read_to_string(
        path.as_str(),
    )?))
}

#[shiika_method("Meta:File#write")]
pub extern "C" fn meta_file_write(
    _receiver: SkClass,
    path: SkString,
    content: SkString,
) -> SkResult<SkVoid> {
    fs::write(path.as_str(), content.value()).into()
}

shiika_method_ref!(
    "Meta:File#new",
    fn(receiver: SkClass, path: SkString, file: *const u8) -> SkFile,
    "meta_file_new"
);

extern "C" fn file_finalizer(obj: *mut c_void, _data: *mut c_void) {
    let shiika_file = obj as *mut ShiikaFile;
    unsafe {
        let raw = (*shiika_file).buf_reader_ptr;
        if !raw.is_null() {
            // Drop the heap-allocated BufReader<File>.
            drop(Box::from_raw(raw));
            (*shiika_file).buf_reader_ptr = ptr::null_mut();
        }
    }
}

#[allow(non_snake_case)]
#[shiika_method("Meta:File#_open")]
pub extern "C" fn meta_file__open(cls_file: SkClass, path: SkString) -> SkResult<SkFile> {
    _meta_file_open(cls_file, path).into()
}

fn _meta_file_open(cls_file: SkClass, path: SkString) -> Result<SkFile, std::io::Error> {
    let file = File::open(path.as_str())?;
    let mut f = meta_file_new(cls_file, path, ptr::null());
    unsafe {
        bdwgc_alloc::Allocator::register_finalizer(
            f.0 as *const c_void,
            file_finalizer,
            ptr::null(),
        );
    }
    f.set_buf_reader(BufReader::new(file));
    Ok(f)
}

#[shiika_method("File#_fill_buf")]
pub extern "C" fn file_fill_buf(mut sk_file: SkFile) -> SkResult<SkString> {
    match sk_file.buf_reader_mut().fill_buf() {
        Ok(u8slice) => SkResult::ok(SkString::from_vec(u8slice.to_vec())),
        Err(e) => SkResult::fail(format!("{}", e)),
    }
}

#[shiika_method("File#_consume")]
pub extern "C" fn file_consume(mut sk_file: SkFile, n_bytes: SkInt) {
    sk_file.buf_reader_mut().consume(n_bytes.val() as usize);
}
