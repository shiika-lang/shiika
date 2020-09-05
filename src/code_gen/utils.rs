/// Provides utility functions used by code_gen/*.rs
/// (some are also used by corelib/*.rs)
use crate::code_gen::*;
use inkwell::types::*;
use inkwell::AddressSpace;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    pub fn build_ivar_load<'a>(
        &'a self,
        object: inkwell::values::BasicValueEnum<'a>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'a> {
        let ptr = self
            .builder
            .build_struct_gep(
                object.into_pointer_value(),
                idx as u32,
                &format!("addr_{}", name),
            )
            .unwrap();
        self.builder.build_load(ptr, name)
    }

    pub fn build_ivar_store<'a>(
        &'a self,
        object: &'a inkwell::values::BasicValueEnum<'a>,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'a>,
        name: &str,
    ) {
        let ptr = self
            .builder
            .build_struct_gep(
                object.into_pointer_value(),
                idx as u32,
                &format!("addr_{}", name),
            )
            .unwrap();
        self.builder.build_store(ptr, value);
    }

    /// Generate call of GC_malloc and returns a ptr to Shiika object
    pub fn allocate_sk_obj(
        &self,
        class_fullname: &ClassFullname,
        reg_name: &str,
    ) -> inkwell::values::BasicValueEnum<'ictx> {
        let object_type = self.llvm_struct_types.get(&class_fullname).unwrap();
        let obj_ptr_type = object_type.ptr_type(AddressSpace::Generic);
        let size = object_type
            .size_of()
            .expect("[BUG] object_type has no size");

        // %mem = call i8* @GC_malloc(i64 %size)",
        let func = self.get_llvm_func("GC_malloc");
        let raw_addr = self
            .builder
            .build_call(func, &[size.as_basic_value_enum()], "mem")
            .try_as_basic_value()
            .left()
            .unwrap();

        // %foo = bitcast i8* %mem to %#{t}*",
        self.builder.build_bitcast(raw_addr, obj_ptr_type, reg_name)
    }

    pub fn llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum<'ictx> {
        if ty.body == TyBody::TyRaw && ty.fullname.0 == "Shiika::Internal::Ptr" {
            self.i8ptr_type.as_basic_type_enum()
        } else {
            self.sk_obj_llvm_type(ty)
        }
    }

    /// Return zero value in LLVM. None if it is a pointer
    pub(super) fn llvm_zero_value(&self, _ty: &TermTy) -> Option<inkwell::values::BasicValueEnum> {
        // Currently all values are pointer
        None
    }

    /// Helper func for self.llvm_type()
    fn sk_obj_llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum<'ictx> {
        let s = match &ty.body {
            TyBody::TySpe { base_name, .. } => &base_name,
            TyBody::TyParamRef { .. } => "Object", // its upper bound
            _ => &ty.fullname.0,
        };
        let struct_type = self
            .llvm_struct_types
            .get(&class_fullname(s))
            .unwrap_or_else(|| panic!("[BUG] struct_type not found: {:?}", ty.fullname));
        struct_type
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum()
    }

    /// Return the llvm func
    /// Panic if not found
    pub(super) fn get_llvm_func(&self, name: &str) -> inkwell::values::FunctionValue<'ictx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("[BUG] get_llvm_func: `{:?}' not found", name))
    }
}
