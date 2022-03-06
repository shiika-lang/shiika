/// Shiika object (eg. `Int*`, `String*`)
#[derive(Clone, Debug)]
pub struct SkObj<'run>(pub inkwell::values::BasicValueEnum<'run>);

impl<'run> SkObj<'run> {
    /// A class object is a Shiika object.
    pub fn as_class_obj(self) -> SkModuleObj<'run> {
        SkModuleObj(self.0)
    }
}

/// Shiika class object (eg. `Meta:Int*`, `Meta:String*`)
#[derive(Debug)]
pub struct SkModuleObj<'run>(pub inkwell::values::BasicValueEnum<'run>);

impl<'run> SkModuleObj<'run> {
    /// A class object is a Shiika object.
    pub fn as_sk_obj(self) -> SkObj<'run> {
        SkObj(self.0)
    }
}

/// Reference to vtable (eg. `shiika_vtable_Int`)
#[derive(Debug)]
pub struct VTableRef<'run>(pub inkwell::values::BasicValueEnum<'run>);

impl<'run> VTableRef<'run> {
    /// Normally vtables are not Shiika object. This is used internally
    pub fn as_sk_obj(self) -> SkObj<'run> {
        SkObj(self.0)
    }
}

/// i8*
#[derive(Debug)]
pub struct I8Ptr<'run>(pub inkwell::values::PointerValue<'run>);
