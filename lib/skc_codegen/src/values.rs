use crate::CodeGen;

/// Shiika object (eg. `Int*`, `String*`)
#[derive(Clone, Debug)]
pub struct SkObj<'run>(pub inkwell::values::BasicValueEnum<'run>);

impl<'run> SkObj<'run> {
    /// A class object is a Shiika object.
    pub fn as_class_obj(self) -> SkClassObj<'run> {
        SkClassObj(self.0)
    }

    /// Bitcast this object to i8*
    pub fn into_i8ptr(
        self,
        code_gen: &CodeGen<'_, 'run, '_>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        code_gen
            .builder
            .build_bitcast(self.0, code_gen.i8ptr_type, "ptr")
    }
}

/// Shiika class object (eg. `Meta:Int*`, `Meta:String*`)
#[derive(Debug)]
pub struct SkClassObj<'run>(pub inkwell::values::BasicValueEnum<'run>);

impl<'run> SkClassObj<'run> {
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
