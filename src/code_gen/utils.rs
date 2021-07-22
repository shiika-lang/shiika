use crate::code_gen::*;
/// Provides utility functions used by code_gen/*.rs
/// (some are also used by corelib/*.rs)
use inkwell::types::*;
use inkwell::AddressSpace;

/// Number of elements before ivars
const OBJ_HEADER_SIZE: usize = 2;
/// 0th: reference to the vtable
const OBJ_VTABLE_IDX: usize = 0;
/// 1st: reference to the class object
const OBJ_CLASS_IDX: usize = 1;

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

    /// Get the vtable of an object as i8ptr
    pub fn get_vtable_of_obj(
        &self,
        object: inkwell::values::BasicValueEnum<'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        self.build_llvm_struct_ref(object, OBJ_VTABLE_IDX, "vtable")
    }

    /// Get the class object of an object as `*Class`
    pub fn get_class_of_obj(
        &self,
        object: inkwell::values::BasicValueEnum<'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let i8ptr = self.build_llvm_struct_ref(object, OBJ_CLASS_IDX, "class");
        let class_type = self.llvm_struct_type(&class_fullname("Class"));
        self.builder.build_bitcast(
            i8ptr,
            class_type.ptr_type(AddressSpace::Generic),
            "sk_class",
        )
    }

    /// Set `class_obj` to the class object field of `object`
    pub fn set_class_of_obj(
        &self,
        object: &inkwell::values::BasicValueEnum<'run>,
        class_obj: inkwell::values::BasicValueEnum<'run>,
    ) {
        let cast =
            self.builder
                .build_bitcast(class_obj, self.llvm_type(&ty::raw("Class")), "class");
        self.build_llvm_struct_set(object, OBJ_CLASS_IDX, cast, "my_class");
    }

    /// Set `vtable` to `object`
    pub fn set_vtable_of_obj(
        &self,
        object: &inkwell::values::BasicValueEnum<'run>,
        vtable: inkwell::values::BasicValueEnum<'run>,
    ) {
        self.build_llvm_struct_set(object, OBJ_VTABLE_IDX, vtable, "vtable");
    }

    /// Get vtable of the class of the given name as *i8
    pub fn vtable_ref(&self, classname: &ClassFullname) -> inkwell::values::BasicValueEnum<'run> {
        let vtable_const_name = llvm_vtable_const_name(classname);
        let llvm_ary_ptr = self
            .module
            .get_global(&vtable_const_name)
            .unwrap_or_else(|| panic!("[BUG] global `{}' not found", &vtable_const_name))
            .as_pointer_value();
        self.into_i8ptr(llvm_ary_ptr)
    }

    /// Lookup llvm func from vtable of an object
    pub fn build_vtable_ref(
        &self,
        vtable_i8ptr: inkwell::values::PointerValue<'run>,
        idx: usize,
        size: usize,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let ary_type = self.i8ptr_type.array_type(size as u32);
        let vtable_ptr = self
            .builder
            .build_bitcast(
                vtable_i8ptr,
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

    /// Load value of nth element of llvm struct
    fn build_llvm_struct_ref(
        &self,
        object: inkwell::values::BasicValueEnum<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let obj_ptr = object.into_pointer_value();
        let obj_ptr_ty = obj_ptr.get_type();
        let ptr = self
            .builder
            .build_struct_gep(
                obj_ptr,
                idx as u32,
                &format!("addr_{}", name),
            )
            .unwrap_or_else(|_| {
                let pointee_ty = obj_ptr_ty.get_element_type();
                panic!(
                    "build_llvm_struct_ref: elem not found (idx: {}, name: {}, pointee_ty: {:?}, object: {:?})",
                    &idx, &name, &pointee_ty, &object
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
        let class_const_name = llvm_class_const_name(class_fullname);
        let class_obj = self
            .module
            .get_global(&class_const_name)
            .unwrap_or_else(|| panic!("global `{}' not found", class_const_name))
            .as_basic_value_enum();
        self._allocate_sk_obj(class_fullname, reg_name, class_obj)
    }

    pub fn _allocate_sk_obj(
        &self,
        class_fullname: &ClassFullname,
        reg_name: &str,
        class_obj: inkwell::values::BasicValueEnum,
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
        self.set_vtable_of_obj(&obj, self.vtable_ref(class_fullname));
        // Store reference to class obj
        self.set_class_of_obj(&obj, class_obj);

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

    /// Cast `sk_obj` to `i8*`
    pub(super) fn into_i8ptr<V>(&self, sk_obj: V) -> inkwell::values::BasicValueEnum<'run>
    where
        V: inkwell::values::BasicValue<'run>,
    {
        self.builder.build_bitcast(sk_obj, self.i8ptr_type, "i8ptr")
    }
}

/// Name of llvm constant of a vtable
pub(super) fn llvm_vtable_const_name(classname: &ClassFullname) -> String {
    format!("shiika_vtable_{}", classname.0)
}

/// Name of llvm constant of a class object
fn llvm_class_const_name(classname: &ClassFullname) -> String {
    if classname.is_meta() {
        "::Class".to_string()
    } else {
        format!("::{}", classname.0)
    }
}
