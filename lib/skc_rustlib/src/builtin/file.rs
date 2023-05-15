use crate::builtin::{SkClass, SkResult, SkStr, SkVoid};
use libc::c_void;
use shiika_ffi_macro::{shiika_method, shiika_method_ref};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::ptr;

#[shiika_method("Meta:File#read")]
pub extern "C" fn meta_file_read(_receiver: SkClass, path: SkStr) -> SkResult<SkStr> {
    // TODO: Support reading binary (i.e. non-utf8) files by using [u8]
    match fs::read_to_string(path.as_str()) {
        Ok(content) => SkResult::ok(SkStr::new(content)),
        Err(e) => SkResult::fail(format!("{}", e)),
    }
}

#[shiika_method("Meta:File#write")]
pub extern "C" fn meta_file_write(
    _receiver: SkClass,
    path: SkStr,
    content: SkStr,
) -> SkResult<SkVoid> {
    fs::write(path.as_str(), content.as_byteslice()).into()
}

shiika_method_ref!(
    "Meta:File#new",
    fn(receiver: SkClass, path: SkStr, file: *const u8) -> SkFile,
    "meta_file_new"
);

#[repr(C)]
#[derive(Debug)]
pub struct SkFile(*mut ShiikaFile);

#[repr(C)]
#[derive(Debug)]
struct ShiikaFile {
    vtable: *const u8,
    class_obj: *const u8,
    file: Option<File>,
}

extern "C" fn file_finalizer(obj: *mut c_void, _data: *mut c_void) {
    println!("[file_finalizer {:?}]", obj);
    let shiika_file = obj as *mut ShiikaFile;
    file_close(SkFile(shiika_file))
}

#[allow(non_snake_case)]
#[shiika_method("Meta:File#_open")]
pub extern "C" fn meta_file__open(the_file: SkClass, path: SkStr) -> SkFile {
    let file = File::open(&path.as_str()).unwrap(); // TODO: use SkResult
    unsafe {
        let f = meta_file_new(the_file, path.dup(), ptr::null());
        println!("[f: {:?}]", f);
        bdwgc_alloc::Allocator::register_finalizer(
            f.0 as *const c_void,
            file_finalizer,
            ptr::null(),
        );
        (*f.0).file = Some(file);
        f
    }
}

#[shiika_method("File#read")]
pub extern "C" fn file_read(sk_file: SkFile) -> SkStr {
    let file = unsafe { (*sk_file.0).file.as_ref().unwrap() };
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents).unwrap(); // TODO: use SkResult
    contents.into()
}

#[shiika_method("File#close")]
pub extern "C" fn file_close(sk_file: SkFile) {
    unsafe { (*sk_file.0).file = None }
}
