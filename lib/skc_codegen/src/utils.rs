use crate::values::*;
use crate::CodeGen;
use inkwell::types::*;
use inkwell::values::*;
use inkwell::AddressSpace;
use shiika_core::{names::*, ty, ty::*};

/// Number of elements before ivars
const OBJ_HEADER_SIZE: usize = 2;
/// 0th: reference to the vtable
const OBJ_VTABLE_IDX: usize = 0;
/// 1st: reference to the class object
const OBJ_CLASS_IDX: usize = 1;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Build IR to return Shiika object
    pub fn build_return(&self, obj: &SkObj<'run>) {
        self.builder.build_return(Some(&obj.0));
    }

    /// Build IR to return ::Void
    pub fn build_return_void(&self) {
        let v = self.gen_const_ref(&toplevel_const("Void"));
        self.build_return(&v);
    }

    /// Load value of an instance variable
    pub fn build_ivar_load(&self, object: SkObj<'run>, idx: usize, name: &str) -> SkObj<'run> {
        SkObj(self.build_llvm_struct_ref(object, OBJ_HEADER_SIZE + idx, name))
    }

    /// Store value into an instance variable
    pub fn build_ivar_store<'a>(
        &'a self,
        object: &'a SkObj<'a>,
        idx: usize,
        value: SkObj<'a>,
        name: &str,
    ) {
        self.build_ivar_store_raw(object, idx, value.0, name)
    }

    /// Store llvm value into an instance variable
    pub fn build_ivar_store_raw<'a>(
        &'a self,
        object: &'a SkObj<'a>,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'a>,
        name: &str,
    ) {
        self.build_llvm_struct_set(object, OBJ_HEADER_SIZE + idx, value, name)
    }

    /// Get the vtable of an object as i8ptr
    pub fn get_vtable_of_obj(&self, object: SkObj<'run>) -> VTableRef<'run> {
        VTableRef(self.build_llvm_struct_ref(object, OBJ_VTABLE_IDX, "vtable"))
    }

    /// Get the class object of an object as `*Class`
    pub fn get_class_of_obj(&self, object: SkObj<'run>) -> SkClassObj<'run> {
        SkClassObj(self.build_llvm_struct_ref(object, OBJ_CLASS_IDX, "class"))
    }

    /// Set `class_obj` to the class object field of `object`
    pub fn set_class_of_obj(&self, object: &SkObj<'run>, class_obj: SkClassObj<'run>) {
        let cast = self.bitcast(SkObj(class_obj.0), &ty::raw("Class"), "class");
        self.build_llvm_struct_set(object, OBJ_CLASS_IDX, cast.0, "my_class");
    }

    /// Set `vtable` to `object`
    pub fn set_vtable_of_obj(&self, object: &SkObj<'run>, vtable: VTableRef<'run>) {
        self.build_llvm_struct_set(object, OBJ_VTABLE_IDX, vtable.0, "vtable");
    }

    /// Get vtable of the class of the given name
    fn get_vtable_of_class(&self, classname: &ClassFullname) -> VTableRef<'run> {
        let vtable_const_name = llvm_vtable_const_name(classname);
        let llvm_ary_ptr = self
            .module
            .get_global(&vtable_const_name)
            .unwrap_or_else(|| panic!("[BUG] global `{}' not found", &vtable_const_name))
            .as_pointer_value();
        VTableRef(
            self.builder
                .build_bitcast(llvm_ary_ptr, self.i8ptr_type, "i8ptr"),
        )
    }

    /// Lookup llvm func from vtable of an object
    pub fn build_vtable_ref(
        &self,
        vtable_ref: VTableRef<'run>,
        idx: usize,
        size: usize,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let ary_type = self.i8ptr_type.array_type(size as u32);
        let vtable_ptr = self
            .builder
            .build_bitcast(
                vtable_ref.0,
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
        object: SkObj<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let obj_ptr = object.0.into_pointer_value();
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
        object: &'a SkObj<'a>,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'a>,
        name: &str,
    ) {
        let ptr = self
            .builder
            .build_struct_gep(
                object.0.into_pointer_value(),
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
    pub fn allocate_sk_obj(&self, class_fullname: &ClassFullname, reg_name: &str) -> SkObj<'run> {
        let class_obj = self.load_class_object(class_fullname);
        self._allocate_sk_obj(class_fullname, reg_name, class_obj)
    }

    /// Load a class object
    pub fn load_class_object(&self, class_fullname: &ClassFullname) -> SkClassObj<'run> {
        let class_const_name = format!("::{}", class_fullname.0);
        let class_obj_addr = self
            .module
            .get_global(&class_const_name)
            .unwrap_or_else(|| panic!("global `{}' not found", class_const_name))
            .as_pointer_value();
        SkClassObj(self.builder.build_load(class_obj_addr, "class_obj"))
    }

    pub fn _allocate_sk_obj(
        &self,
        class_fullname: &ClassFullname,
        reg_name: &str,
        class_obj: SkClassObj,
    ) -> SkObj<'run> {
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
        let obj = SkObj(self.builder.build_bitcast(raw_addr, obj_ptr_type, reg_name));

        // Store reference to vtable
        self.set_vtable_of_obj(&obj, self.get_vtable_of_class(class_fullname));
        // Store reference to class obj
        self.set_class_of_obj(&obj, class_obj);

        obj
    }

    /// Call llvm function which corresponds to a Shiika method
    pub fn call_method_func(
        &self,
        func_name: &str,
        receiver: SkObj<'run>,
        args: &[SkObj<'run>],
        reg_name: &str,
    ) -> SkObj<'run> {
        let mut llvm_args = vec![receiver.0];
        llvm_args.append(&mut args.iter().map(|x| x.0).collect());
        SkObj(self.call_llvm_func(func_name, &llvm_args, reg_name))
    }

    /// Call llvm function (whose return type is not `void`)
    pub fn call_llvm_func(
        &self,
        func_name: &str,
        args: &[inkwell::values::BasicValueEnum<'run>],
        reg_name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let f = self
            .module
            .get_function(func_name)
            .unwrap_or_else(|| panic!("[BUG] llvm function `{}' not found", func_name));
        self.builder
            .build_call(f, args, reg_name)
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    /// Call llvm function whose return type is `void`
    pub fn call_llvm_void_func(
        &self,
        func_name: &str,
        args: &[inkwell::values::BasicValueEnum<'run>],
    ) {
        let f = self.module.get_function(func_name).unwrap();
        self.builder.build_call(f, args, "");
    }

    /// Get nth parameter of llvm func as SkObj
    pub fn get_nth_param(
        &self,
        function: &inkwell::values::FunctionValue<'run>,
        n: usize,
    ) -> SkObj<'run> {
        SkObj(function.get_nth_param(n as u32).unwrap())
    }

    /// Cast an object to different Shiika type
    pub fn bitcast(&self, obj: SkObj<'run>, ty: &TermTy, reg_name: &str) -> SkObj<'run> {
        SkObj(
            self.builder
                .build_bitcast(obj.0, self.llvm_type(ty), reg_name),
        )
    }

    /// LLVM type of a Shiika object
    pub fn llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum<'ictx> {
        let s = match &ty.body {
            TyBody::TyParamRef { upper_bound, .. } => return self.llvm_type(upper_bound),
            TyBody::TyRaw { base_name, .. } => base_name,
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
    pub(super) fn get_llvm_func(&self, name: &str) -> inkwell::values::FunctionValue<'run> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("[BUG] get_llvm_func: `{:?}' not found", name))
    }
}

/// Name of llvm constant of a vtable
pub(super) fn llvm_vtable_const_name(classname: &ClassFullname) -> String {
    format!("shiika_vtable_{}", classname.0)
}
