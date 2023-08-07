use crate::utils::OBJ_VTABLE_IDX;
use crate::values::SkObj;
use crate::CodeGen;
use inkwell::types::BasicType;
use inkwell::values::BasicValue;
use shiika_core::ty;

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
        Self::llvm_pointee_type(gen, len).ptr_type(Default::default())
    }

    fn llvm_pointee_type(
        gen: &CodeGen<'_, 'run, '_>,
        len: usize,
    ) -> inkwell::types::ArrayType<'run> {
        gen.i8ptr_type.array_type(len as u32)
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
            .build_load(
                Self::llvm_pointee_type(gen, self.len),
                self.ptr.clone(),
                "vtable",
            )
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
