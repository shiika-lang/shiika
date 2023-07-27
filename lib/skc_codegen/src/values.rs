use crate::CodeGen;
use inkwell::values::BasicValue;

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
pub struct VTableRef<'run> {
    pub ptr: inkwell::values::PointerValue<'run>,
    len: usize,
}

impl<'run> VTableRef<'run> {
    pub fn new(ptr: inkwell::values::PointerValue<'run>, len: usize) -> VTableRef<'run> {
        VTableRef { ptr, len }
    }

    pub fn get_func(
        &self,
        gen: &CodeGen<'_, 'run, '_>,
        idx: usize,
    ) -> inkwell::values::BasicValueEnum<'run> {
        gen.builder
            .build_extract_value(self.get_vtable(gen), idx as u32, "func_raw")
            .unwrap()
    }

    fn get_vtable(&self, gen: &CodeGen<'_, 'run, '_>) -> inkwell::values::ArrayValue<'run> {
        gen.builder
            .build_load(self.llvm_type(gen), self.ptr.clone(), "vtable")
            .into_array_value()
    }

    fn llvm_type(&self, gen: &CodeGen<'_, 'run, '_>) -> inkwell::types::PointerType<'run> {
        let ary_type = gen.i8ptr_type.array_type(self.len as u32);
        ary_type.ptr_type(Default::default())
    }
}

/// Reference to vtable where its length is unknown.
#[derive(Debug)]
pub struct OpaqueVTableRef<'run> {
    pub ptr: inkwell::values::PointerValue<'run>,
}

impl<'run> OpaqueVTableRef<'run> {
    pub fn new(ptr: inkwell::values::PointerValue<'run>) -> OpaqueVTableRef<'run> {
        OpaqueVTableRef { ptr }
    }

    /// Normally vtables are not Shiika object. This is used internally
    pub fn as_sk_obj(self) -> SkObj<'run> {
        SkObj(self.ptr.as_basic_value_enum())
    }
}

impl<'run> From<VTableRef<'run>> for OpaqueVTableRef<'run> {
    fn from(x: VTableRef) -> Self {
        OpaqueVTableRef::new(x.ptr)
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
