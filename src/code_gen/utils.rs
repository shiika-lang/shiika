/// Provides utility functions used by code_gen/*.rs
/// (some are also used by corelib/*.rs)
use crate::code_gen::*;
use inkwell::types::*;
use inkwell::AddressSpace;

/// Number of elements before ivars
const OBJ_HEADER_SIZE: usize = 1;
/// 0th: reference to vtable
const OBJ_VTABLE_IDX: usize = 0;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Build IR to return ::Void
    pub fn build_return_void(&self) {
        let v = self.gen_const_ref(&toplevel_const("Void"));
        self.builder.build_return(Some(&v));
    }

    /// Load value of an instance variable
    pub fn build_ivar_load(
        &self,
        object: inkwell::values::BasicValueEnum<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        self.build_llvm_struct_ref(object, OBJ_HEADER_SIZE + idx, name)
    }

    /// Store value into an instance variable
    pub fn build_ivar_store<'a>(
        &'a self,
        object: &'a inkwell::values::BasicValueEnum<'a>,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'a>,
        name: &str,
    ) {
        self.build_llvm_struct_set(object, OBJ_HEADER_SIZE + idx, value, name)
    }

    /// Lookup llvm func from vtable of an object
    pub fn build_vtable_ref(
        &self,
        object: inkwell::values::BasicValueEnum<'run>,
        idx: usize,
        size: usize,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let vtable_ref = self.build_llvm_struct_ref(object, OBJ_VTABLE_IDX, "vtable_ref");
        let ary_type = self.i8ptr_type.array_type(size as u32);
        let vtable_ptr = self
            .builder
            .build_bitcast(
                vtable_ref,
                ary_type.ptr_type(AddressSpace::Generic),
                "vtable_ptr",
            )
            .into_pointer_value();
        let vtable = self
            .builder
            .build_load(vtable_ptr, "vtable")
            .into_array_value();
        self.builder
            .build_extract_value(vtable, idx as u32, "func_raw")
            .unwrap()
    }

    /// Store reference to vtable into an object
    pub fn build_store_vtable<'a>(
        &'a self,
        object: inkwell::values::BasicValueEnum<'a>,
        class_fullname: &ClassFullname,
    ) {
        let vtable_ref = self
            .module
            .get_global(&llvm_vtable_name(class_fullname))
            .unwrap()
            .as_pointer_value();
        let vtable = self
            .builder
            .build_bitcast(vtable_ref, self.i8ptr_type, "vtable");
        self.build_llvm_struct_set(&object, OBJ_VTABLE_IDX, vtable, "vtable")
    }

    /// Load value of nth element of llvm struct
    fn build_llvm_struct_ref(
        &self,
        object: inkwell::values::BasicValueEnum<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let ptr = self
            .builder
            .build_struct_gep(
                object.into_pointer_value(),
                idx as u32,
                &format!("addr_{}", name),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "build_llvm_struct_ref: elem not found (idx: {}, name: {}, object: {:?})",
                    &idx, &name, &object
                )
            });
        self.builder.build_load(ptr, name)
    }

    /// Set value to nth element of llvm struct
    fn build_llvm_struct_set<'a>(
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
            .unwrap_or_else(|_| {
                panic!(
                    "build_llvm_struct_ref: elem not found (idx: {}, name: {}, object: {:?})",
                    &idx, &name, &object
                )
            });
        self.builder.build_store(ptr, value);
    }

    /// Generate call of malloc and returns a ptr to Shiika object
    pub fn allocate_sk_obj(
        &self,
        class_fullname: &ClassFullname,
        reg_name: &str,
    ) -> inkwell::values::BasicValueEnum<'ictx> {
        let object_type = self.llvm_struct_type(class_fullname);
        let obj_ptr_type = object_type.ptr_type(AddressSpace::Generic);
        let size = object_type
            .size_of()
            .expect("[BUG] object_type has no size");

        // %mem = call i8* @shiika_malloc(i64 %size)",
        let func = self.get_llvm_func("shiika_malloc");
        let raw_addr = self
            .builder
            .build_call(func, &[size.as_basic_value_enum()], "mem")
            .try_as_basic_value()
            .left()
            .unwrap();

        // %foo = bitcast i8* %mem to %#{t}*",
        let obj = self.builder.build_bitcast(raw_addr, obj_ptr_type, reg_name);

        // Store reference to vtable
        self.build_store_vtable(obj, class_fullname);

        obj
    }

    /// Return zero value in LLVM. None if it is a pointer
    pub(super) fn llvm_zero_value(&self, _ty: &TermTy) -> Option<inkwell::values::BasicValueEnum> {
        // Currently all values are pointer
        None
    }

    /// LLVM type of a Shiika object
    pub fn llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum<'ictx> {
        let s = match &ty.body {
            TyBody::TySpe { base_name, .. } => base_name,
            TyBody::TyParamRef { .. } => "Object", // its upper bound
            _ => &ty.fullname.0,
        };
        self.llvm_struct_type(&class_fullname(s))
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum()
    }

    /// Get the llvm struct type for a class
    fn llvm_struct_type(&self, name: &ClassFullname) -> &inkwell::types::StructType<'ictx> {
        self.llvm_struct_types
            .get(name)
            .unwrap_or_else(|| panic!("[BUG] struct_type not found: {:?}", name))
    }

    /// Return the llvm func
    /// Panic if not found
    pub(super) fn get_llvm_func(&self, name: &str) -> inkwell::values::FunctionValue<'ictx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("[BUG] get_llvm_func: `{:?}' not found", name))
    }
}

/// Name of llvm constant of a vtable
pub(super) fn llvm_vtable_name(classname: &ClassFullname) -> String {
    format!("vtable_{}", classname)
}
