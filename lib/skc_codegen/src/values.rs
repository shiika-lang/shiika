use crate::utils::OBJ_VTABLE_IDX;
use crate::CodeGen;
use inkwell::types::BasicType;
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

    pub fn ivar_store(&self, gen: &CodeGen<'_, 'run, '_>, name: &str, value: SkObj<'run>) {
        self.ivar_store_raw(gen, name, value.0.as_basic_value_enum());
    }

    pub fn ivar_store_raw(
        &self,
        gen: &CodeGen<'_, 'run, '_>,
        name: &str,
        value: inkwell::values::BasicValueEnum<'run>,
    ) {
        let sk_class = gen
            .sk_types
            .get_class(&self.1.erasure().to_class_fullname());
        let sk_ivar = sk_class.ivars.get(name).unwrap();
        gen.build_llvm_struct_set(
            &self.struct_ty(gen),
            self.0.clone(),
            sk_ivar.idx,
            value,
            name,
        );
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

    fn llvm_type(gen: &CodeGen<'_, 'run, '_>, len: usize) -> inkwell::types::PointerType<'run> {
        gen.i8ptr_type
            .array_type(len as u32)
            .ptr_type(Default::default())
    }

    /// Returns the vtable of a Shiika object.
    pub fn from_sk_obj(
        gen: &CodeGen<'_, 'run, '_>,
        object: SkObj<'run>,
        len: usize,
    ) -> VTableRef<'run> {
        let item_ty = Self::llvm_type(gen, len).as_basic_type_enum();
        let ptr = gen
            .build_object_struct_ref_raw(object, item_ty, OBJ_VTABLE_IDX, "vtable")
            .into_pointer_value();
        VTableRef::new(ptr, len)
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
            .build_load(Self::llvm_type(gen, self.len), self.ptr.clone(), "vtable")
            .into_array_value()
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
    pub fn as_object_ptr(self, gen: &CodeGen<'_, 'run, '_>) -> SkObj<'run> {
        let ty = ty::raw("Object");
        SkObj::new(
            ty.clone(),
            gen.builder
                .build_bitcast(self.ptr.as_basic_value_enum(), gen.llvm_type(&ty), "as"),
        )
    }
}

impl<'run> From<VTableRef<'run>> for OpaqueVTableRef<'run> {
    fn from(x: VTableRef<'run>) -> Self {
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
