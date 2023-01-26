use crate::builtin::object::ShiikaObject;
use crate::builtin::SkObj;
use shiika_ffi_macro::shiika_const_ref;

shiika_const_ref!("::Void", SkVoid, "sk_Void");

#[repr(C)]
#[derive(Debug)]
pub struct SkVoid(*const ShiikaVoid);

impl From<()> for SkVoid {
    fn from(_: ()) -> Self {
        sk_Void()
    }
}

impl From<SkVoid> for SkObj {
    fn from(s: SkVoid) -> SkObj {
        SkObj::new(s.0 as *const ShiikaObject)
    }
}

impl SkVoid {
    pub fn dup(&self) -> SkVoid {
        SkVoid(self.0)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShiikaVoid {
    vtable: *const u8,
    class_obj: *const u8,
}
