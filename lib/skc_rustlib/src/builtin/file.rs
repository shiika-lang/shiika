use crate::builtin::object::ShiikaObject;
use crate::builtin::{SkClass, SkInt, SkObj, SkResult, SkStr, SkVoid};
use libc::c_void;
use shiika_ffi_macro::{shiika_method, shiika_method_ref};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ptr;

#[shiika_method("Meta:File#read")]
pub extern "C" fn meta_file_read(_receiver: SkClass, path: SkStr) -> SkResult<SkStr> {
    // TODO: Support reading binary (i.e. non-utf8) files by using [u8]
    _meta_file_read(path).into()
}

fn _meta_file_read(path: SkStr) -> Result<SkStr, std::io::Error> {
    Ok(SkStr::new(fs::read_to_string(path.as_str())?))
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
    buf_reader: BufReader<File>,
}

impl From<SkFile> for SkObj {
    fn from(s: SkFile) -> SkObj {
        SkObj::new(s.0 as *const ShiikaObject)
    }
}

impl SkFile {
    fn buf_reader_mut(&mut self) -> &mut BufReader<File> {
        unsafe { &mut (*self.0).buf_reader }
    }

    fn file_mut(&mut self) -> &mut File {
        self.buf_reader_mut().get_mut()
    }
}

extern "C" fn file_finalizer(obj: *mut c_void, _data: *mut c_void) {
    let shiika_file = obj as *mut ShiikaFile;
    std::mem::drop(SkFile(shiika_file).file_mut());
}

#[allow(non_snake_case)]
#[shiika_method("Meta:File#_open")]
pub extern "C" fn meta_file__open(cls_file: SkClass, path: SkStr) -> SkResult<SkFile> {
    _meta_file_open(cls_file, path).into()
}

fn _meta_file_open(cls_file: SkClass, path: SkStr) -> Result<SkFile, std::io::Error> {
    let file = File::open(&path.as_str())?;
    unsafe {
        let f = meta_file_new(cls_file, path.dup(), ptr::null());
        bdwgc_alloc::Allocator::register_finalizer(
            f.0 as *const c_void,
            file_finalizer,
            ptr::null(),
        );
        (*f.0).buf_reader = BufReader::new(file);
        Ok(f)
    }
}

//#[shiika_method("File#close")]
//pub extern "C" fn file_close(sk_file: SkFile) {
//    unsafe { (*sk_file.0).file = None }
//}

#[shiika_method("File#_fill_buf")]
pub extern "C" fn file_fill_buf(mut sk_file: SkFile) -> SkResult<SkStr> {
    match sk_file.buf_reader_mut().fill_buf() {
        Ok(u8slice) => SkResult::ok(SkStr::from_u8(u8slice.to_vec())),
        Err(e) => SkResult::fail(format!("{}", e)),
    }
}

#[shiika_method("File#_consume")]
pub extern "C" fn file_consume(mut sk_file: SkFile, n_bytes: SkInt) {
    sk_file.buf_reader_mut().consume(n_bytes.into());
}
