use crate::CodeGen;
use inkwell::values::BasicValue;
use shiika_core::{names::ClassFullname, ty, ty::TermTy};

/// Shiika object (eg. `Int*`, `String*`)
#[derive(Clone, Debug)]
pub struct SkObj<'run>(pub inkwell::values::PointerValue<'run>, TermTy);

impl<'run> SkObj<'run> {
    pub fn new<T>(ty: TermTy, ptr: T) -> SkObj<'run>
    where
        T: Into<inkwell::values::BasicValueEnum<'run>>,
    {
        SkObj(ptr.into().into_pointer_value(), ty)
    }

    /// Returns a null pointer cast to `%Object*`.
    pub fn nullptr(gen: &CodeGen<'_, 'run, '_>) -> SkObj<'run> {
        let ty = ty::raw("Object");
        let null = gen.i8ptr_type.const_null().as_basic_value_enum();
        SkObj::new(
            ty.clone(),
            gen.builder.build_bitcast(null, gen.llvm_type(&ty), "as"),
        )
    }

    pub fn ty(&self) -> &TermTy {
        &self.1
    }

    pub fn classname(&self) -> ClassFullname {
        self.1.erasure().to_class_fullname()
    }

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

    pub fn struct_ty(&self, gen: &'run CodeGen<'_, 'run, '_>) -> inkwell::types::StructType<'run> {
        gen.llvm_struct_type(&self.1).clone()
    }
}

/// Shiika class object (eg. `Meta:Int*`, `Meta:String*`)
#[derive(Debug)]
pub struct SkClassObj<'run>(pub inkwell::values::PointerValue<'run>);

impl<'run> SkClassObj<'run> {
    /// Returns a null pointer cast to `%Object*`.
    pub fn nullptr(gen: &CodeGen<'_, 'run, '_>) -> SkClassObj<'run> {
        let ty = ty::raw("Class");
        let null = gen.i8ptr_type.const_null().as_basic_value_enum();
        SkClassObj(
            gen.builder
                .build_bitcast(null, gen.llvm_type(&ty), "as")
                .into_pointer_value(),
        )
    }

    /// A class object is a Shiika object.
    pub fn as_sk_obj(self) -> SkObj<'run> {
        SkObj::new(ty::raw("Class"), self.0)
    }
}

/// i8* (REFACTOR: rename to VoidPtr)
#[derive(Debug)]
pub struct I8Ptr<'run>(pub inkwell::values::PointerValue<'run>);

impl<'run> I8Ptr<'run> {
    /// Create a void pointer with bitcast
    pub fn cast(
        gen: &CodeGen<'_, 'run, '_>,
        p: inkwell::values::PointerValue<'run>,
    ) -> I8Ptr<'run> {
        I8Ptr(
            gen.builder
                .build_bitcast(p, gen.i8ptr_type, "cast")
                .into_pointer_value(),
        )
    }

    /// Box `self` with `Shiika::Internal::Ptr`
    pub fn boxed(self, gen: &CodeGen<'_, 'run, '_>) -> SkObj<'run> {
        gen.box_i8ptr(self.0.into())
    }

    /// Returns `PointerValue` cast to `t`
    pub fn cast_to(
        self,
        gen: &CodeGen<'_, 'run, '_>,
        t: inkwell::types::PointerType<'run>,
    ) -> inkwell::values::PointerValue<'run> {
        gen.builder
            .build_bitcast(self.0, t, "cast_to")
            .into_pointer_value()
    }
}
