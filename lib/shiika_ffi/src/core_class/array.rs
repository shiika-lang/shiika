#[repr(C)]
#[derive(Debug)]
pub struct SkArray<T>(*mut ShiikaArray<T>);

#[repr(C)]
#[derive(Debug)]
struct ShiikaArray<T> {
    vtable: *const u8,
    class_obj: *const u8,
    vec: *mut Vec<T>,
}
