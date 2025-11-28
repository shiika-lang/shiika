#[repr(C)]
#[derive(Debug)]
pub struct SkArray<T>(*mut ShiikaArray<T>);

unsafe impl<T> Send for SkArray<T> {}

#[repr(C)]
#[derive(Debug)]
struct ShiikaArray<T> {
    vtable: *const u8,
    class_obj: *const u8,
    vec: *mut Vec<T>,
}

impl<T> SkArray<T> {
    ///// Call `Array.new`.
    //pub fn new<U: SkCls>() -> SkAry<U> {
    //    let spe_cls = sk_Array().specialize(vec![U::get_class_object()]);
    //    let sk_ary = meta_array_new(spe_cls);
    //    // Force cast because external function (Meta_Array_new)
    //    // cannot have type a parameter.
    //    SkAry(sk_ary.0 as *mut ShiikaArray<U>)
    //}

    pub fn as_vec(&self) -> &Vec<T> {
        unsafe { (*self.0).vec.as_ref().unwrap() }
    }

    pub fn as_vec_mut(&self) -> &mut Vec<T> {
        unsafe { (*self.0).vec.as_mut().unwrap() }
    }

    pub fn into_vec(&self) -> Vec<T> {
        unsafe { (*self.0).vec.read() }
    }

    /// Replace the contents with `v`.
    /// The original Vec will be free'd by GC.
    pub fn set_vec(&self, v: Vec<T>) {
        unsafe { (*self.0).vec = Box::leak(Box::new(v)) }
    }
}
