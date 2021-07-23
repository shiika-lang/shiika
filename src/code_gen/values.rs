/// Shiika object (eg. `Int*`, `String*`)
#[derive(Debug)]
pub struct SkObj<'ictx>(pub inkwell::values::BasicValueEnum<'ictx>);

/// Shiika class object (eg. `Meta:Int*`, `Meta:String*`)
#[derive(Debug)]
pub struct SkClassObj<'ictx>(pub inkwell::values::BasicValueEnum<'ictx>);

impl<'ictx> SkClassObj<'ictx> {
    /// A class object is a Shiika object.
    pub fn as_sk_obj(self) -> SkObj<'ictx> {
        SkObj(self.0)
    }
}

/// Reference to vtable (eg. `shiika_vtable_Int`)
#[derive(Debug)]
pub struct VTableRef<'ictx>(pub inkwell::values::BasicValueEnum<'ictx>);

/// i8*
#[derive(Debug)]
pub struct I8Ptr<'ictx>(pub inkwell::values::PointerValue<'ictx>);
