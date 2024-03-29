use crate::values::*;
use crate::vtable::OpaqueVTableRef;
use crate::CodeGen;
use inkwell::types::*;
use inkwell::values::BasicValue;
use shiika_core::{names::*, ty, ty::*};
use shiika_ffi::{mangle_const, mangle_method};

/// Number of elements before ivars
const OBJ_HEADER_SIZE: usize = 2;
/// 0th: reference to the vtable
pub const OBJ_VTABLE_IDX: usize = 0;
/// 1st: reference to the class object
const OBJ_CLASS_IDX: usize = 1;

#[derive(Debug)]
pub struct LlvmFuncName(pub String);

pub fn llvm_func_name(name: impl Into<String>) -> LlvmFuncName {
    LlvmFuncName(name.into())
}

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Build IR to return Shiika object
    pub fn build_return(&self, obj: &SkObj<'run>) {
        self.builder.build_return(Some(&obj.0));
    }

    /// Build IR to return ::Void
    pub fn build_return_void(&self) {
        let v = self.gen_const_ref(&toplevel_const("Void"), &ty::raw("Void"));
        self.build_return(&v);
    }

    pub fn build_ivar_load(
        &'run self,
        obj: SkObj<'run>,
        item_ty: TermTy,
        idx: usize,
        name: &str,
    ) -> SkObj<'run> {
        let value = self.build_ivar_load_raw(obj, self.llvm_type(), idx, name);
        SkObj::new(item_ty, value)
    }

    pub fn build_ivar_load_raw(
        &'run self,
        obj: SkObj<'run>,
        item_ty: inkwell::types::BasicTypeEnum<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        self.build_llvm_struct_ref_raw(
            &self.llvm_struct_type(&obj.ty()),
            obj.0.clone(),
            item_ty,
            OBJ_HEADER_SIZE + idx,
            name,
        )
    }

    pub fn build_ivar_store(
        &'run self,
        obj: SkObj<'run>,
        idx: usize,
        name: &str,
        value: SkObj<'run>,
    ) {
        self.build_ivar_store_raw(obj, name, idx, value.0.as_basic_value_enum());
    }

    pub fn build_ivar_store_raw(
        &'run self,
        obj: SkObj<'run>,
        name: &str,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'run>,
    ) {
        self.build_llvm_struct_set(
            &obj.struct_ty(self),
            obj.0.clone(),
            OBJ_HEADER_SIZE + idx,
            value,
            name,
        );
    }

    /// Get the class object of an object as `*Class`
    pub fn get_class_of_obj(&self, object: SkObj<'run>) -> SkClassObj<'run> {
        SkClassObj(
            self.build_object_struct_ref(object, OBJ_CLASS_IDX, "class")
                .into_pointer_value(),
        )
    }

    /// Set `class_obj` to the class object field of `object`
    pub fn set_class_of_obj(&self, object: &SkObj<'run>, class_obj: SkClassObj<'run>) {
        self.build_object_struct_set(
            &ty::raw("Object"),
            object,
            OBJ_CLASS_IDX,
            class_obj.0.as_basic_value_enum(),
            "my_class",
        );
    }

    /// Set `vtable` to `object`
    pub fn set_vtable_of_obj(&self, object: &SkObj<'run>, vtable: OpaqueVTableRef<'run>) {
        let v = vtable.ptr.as_basic_value_enum();
        self.build_object_struct_set(&ty::raw("Object"), object, OBJ_VTABLE_IDX, v, "vtable");
    }

    /// Get vtable of the class of the given name
    pub fn get_vtable_of_class(&self, classname: &ClassFullname) -> OpaqueVTableRef<'run> {
        let vtable_const_name = llvm_vtable_const_name(classname);
        let llvm_ary_ptr = self
            .module
            .get_global(&vtable_const_name)
            .unwrap_or_else(|| panic!("[BUG] global `{}' not found", &vtable_const_name))
            .as_pointer_value();
        OpaqueVTableRef::new(llvm_ary_ptr)
    }

    /// Load value of the nth element of the llvm struct of a Shiika object
    pub fn build_object_struct_ref(
        &self,
        object: SkObj<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let struct_ty = self.llvm_struct_type(object.ty());
        self.build_llvm_struct_ref(&struct_ty, object.0, idx, name)
    }

    /// Load a raw llvm value in a Shiika object
    pub fn build_object_struct_ref_raw(
        &self,
        object: SkObj<'run>,
        item_ty: inkwell::types::BasicTypeEnum<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let struct_ty = self.llvm_struct_type(object.ty());
        let ptr = object.0;
        self.build_llvm_struct_ref_raw(&struct_ty, ptr, item_ty, idx, name)
    }

    /// Load value of the nth element of a llvm struct
    pub fn build_llvm_struct_ref(
        &self,
        struct_ty: &inkwell::types::StructType<'run>,
        struct_ptr: inkwell::values::PointerValue<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        self.build_llvm_struct_ref_raw(struct_ty, struct_ptr, self.llvm_type(), idx, name)
    }

    /// Get a value in an llvm struct
    pub fn build_llvm_struct_ref_raw(
        &self,
        struct_ty: &inkwell::types::StructType<'run>,
        struct_ptr: inkwell::values::PointerValue<'run>,
        item_ty: inkwell::types::BasicTypeEnum<'run>,
        idx: usize,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let ptr = self
            .builder
            .build_struct_gep(
                struct_ty.as_basic_type_enum(),
                struct_ptr,
                idx as u32,
                &format!("addr_{}", name),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "build_llvm_struct_ref_raw: elem not found (idx: {}, name: {}, struct_ptr: {:?})",
                    &idx, &name, &struct_ptr
                )
            });
        self.builder.build_load(item_ty, ptr, name)
    }

    /// Set the value the nth element of llvm struct of a Shiika object
    fn build_object_struct_set<'a>(
        &'a self,
        obj_ty: &TermTy,
        object: &'a SkObj<'a>,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'a>,
        name: &str,
    ) {
        let t = self.llvm_struct_type(obj_ty);
        self.build_llvm_struct_set(t, object.0, idx, value, name);
    }

    /// Set the value the nth element of llvm struct
    pub fn build_llvm_struct_set<'a>(
        &'a self,
        struct_type: &inkwell::types::StructType<'run>,
        struct_ptr: inkwell::values::PointerValue<'a>,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'a>,
        name: &str,
    ) {
        let ptr = self
            .builder
            .build_struct_gep(
                struct_type.as_basic_type_enum(),
                struct_ptr,
                idx as u32,
                &format!("addr_{}", name),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "build_llvm_struct_set: elem not found (idx in struct: {}, register name: {}, struct: {:?})",
                    &idx, &name, &struct_ptr
                )
            });
        self.builder.build_store(ptr, value);
    }

    /// Generate call of malloc and returns a ptr to Shiika object
    pub fn allocate_sk_obj(&self, class_fullname: &ClassFullname) -> SkObj<'run> {
        let class_obj = self.load_class_object(class_fullname);
        self._allocate_sk_obj(class_fullname, class_obj)
    }

    /// Load a class object
    /// NOTE: this does not work with `const_is_obj` classes (i.e. Void, None, etc.)
    fn load_class_object(&self, class_fullname: &ClassFullname) -> SkClassObj<'run> {
        let class_const_name = llvm_const_name(&class_fullname.to_const_fullname());
        let class_obj_addr = self
            .module
            .get_global(&class_const_name)
            .unwrap_or_else(|| panic!("global `{}' not found", class_const_name))
            .as_pointer_value();
        let t = self.llvm_type();
        SkClassObj(
            self.builder
                .build_load(t, class_obj_addr, "class_obj")
                .into_pointer_value(),
        )
    }

    pub fn _allocate_sk_obj(
        &self,
        class_fullname: &ClassFullname,
        class_obj: SkClassObj,
    ) -> SkObj<'run> {
        let object_type = self.get_llvm_struct_type(&class_fullname.to_type_fullname());
        let ptr = self.allocate_llvm_obj(&object_type.as_basic_type_enum());
        let obj = SkObj::new(class_fullname.to_ty(), ptr.into_pointer_value());
        self.set_vtable_of_obj(&obj, self.get_vtable_of_class(class_fullname));
        self.set_class_of_obj(&obj, class_obj);
        obj
    }

    /// Allocate some memory for a value of LLVM type `t`. Returns pointer.
    pub fn allocate_llvm_obj(
        &self,
        t: &inkwell::types::BasicTypeEnum<'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        self.allocate_mem(t).0.as_basic_value_enum()
    }

    /// Allocate some memory for a value of LLVM type `t`. Returns void ptr.
    pub fn allocate_mem(&self, t: &inkwell::types::BasicTypeEnum<'run>) -> I8Ptr<'run> {
        let size = t.size_of().expect("[BUG] type has no size");
        self.shiika_malloc(size)
    }

    /// Call `shiika_malloc`
    pub fn shiika_malloc(&self, size: inkwell::values::IntValue<'run>) -> I8Ptr<'run> {
        let func = self.get_llvm_func(&llvm_func_name("shiika_malloc"));
        I8Ptr(
            self.builder
                .build_direct_call(func, &[size.as_basic_value_enum().into()], "mem")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value(),
        )
    }

    /// Call llvm function which corresponds to a Shiika method
    pub fn call_method_func(
        &self,
        method_name: &MethodFullname,
        receiver: SkObj<'run>,
        args: &[SkObj<'run>],
        result_ty: TermTy,
        reg_name: &str,
    ) -> SkObj<'run> {
        let mut llvm_args = vec![receiver.0.into()];
        llvm_args.append(&mut args.iter().map(|x| x.0.into()).collect());
        SkObj::new(
            result_ty,
            self.call_llvm_func(&method_func_name(method_name), &llvm_args, reg_name),
        )
    }

    /// Call llvm function (whose return type is not `void`)
    pub fn call_llvm_func(
        &self,
        func_name: &LlvmFuncName,
        args: &[inkwell::values::BasicMetadataValueEnum<'run>],
        reg_name: &str,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let f = self
            .module
            .get_function(&func_name.0)
            .unwrap_or_else(|| panic!("[BUG] llvm function {:?} not found", func_name));
        self.builder
            .build_direct_call(f, args, reg_name)
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    /// Call llvm function which returns `void`
    pub fn call_void_llvm_func(
        &self,
        func_name: &LlvmFuncName,
        args: &[inkwell::values::BasicMetadataValueEnum<'run>],
        reg_name: &str,
    ) {
        let f = self
            .module
            .get_function(&func_name.0)
            .unwrap_or_else(|| panic!("[BUG] llvm function {:?} not found", func_name));
        self.builder.build_direct_call(f, args, reg_name);
    }

    /// Get nth parameter of llvm func as SkObj
    pub fn get_nth_param(
        &self,
        ty: TermTy,
        function: &inkwell::values::FunctionValue<'run>,
        n: usize,
    ) -> SkObj<'run> {
        SkObj::new(ty, function.get_nth_param(n as u32).unwrap())
    }

    /// Cast an object to different Shiika type
    pub fn bitcast(&self, obj: SkObj<'run>, ty: &TermTy, reg_name: &str) -> SkObj<'run> {
        //debug_assert!(!self.obviously_wrong_bitcast(&obj.0, &self.llvm_type()));
        SkObj::new(
            ty.clone(),
            self.builder
                .build_bitcast(obj.0, self.llvm_type(), reg_name),
        )
    }

    /// Create `%Foo* null`
    pub fn null_ptr(&self, ty: &TermTy) -> SkObj<'run> {
        let ptr = self.llvm_type().into_pointer_type().const_null();
        SkObj::new(ty.clone(), ptr)
    }

    /// LLVM type of a Shiika object
    pub fn llvm_type(&self) -> inkwell::types::BasicTypeEnum<'ictx> {
        self.ptr_type.as_basic_type_enum()
    }

    /// LLVM struct type of a Shiika object
    pub fn llvm_struct_type(&self, ty: &TermTy) -> &inkwell::types::StructType<'ictx> {
        match &ty.body {
            TyBody::TyRaw(t) => self.get_llvm_struct_type(&t.erasure().to_type_fullname()),
            TyBody::TyPara(TyParamRef {
                upper_bound,
                as_class,
                ..
            }) => {
                if *as_class {
                    self.get_llvm_struct_type(&upper_bound.meta_ty().erasure().to_type_fullname())
                } else {
                    self.get_llvm_struct_type(&upper_bound.erasure().to_type_fullname())
                }
            }
        }
    }

    /// Get the llvm struct type for a class/module
    fn get_llvm_struct_type(&self, name: &TypeFullname) -> &inkwell::types::StructType<'ictx> {
        self.llvm_struct_types
            .get(name)
            .unwrap_or_else(|| panic!("[BUG] struct_type not found: {:?}", name))
    }

    /// Return the llvm func
    /// Panic if not found
    pub(super) fn get_llvm_func(
        &self,
        name: &LlvmFuncName,
    ) -> inkwell::values::FunctionValue<'run> {
        self.module
            .get_function(&name.0)
            .unwrap_or_else(|| panic!("[BUG] get_llvm_func: `{:?}' not found", name))
    }
}

/// Name of llvm struct of lambda captures
pub(super) fn lambda_capture_struct_name(name: &str) -> String {
    format!("shiika_captures_{}", name)
}

/// Name of llvm constant of a vtable
pub(super) fn llvm_vtable_const_name(classname: &ClassFullname) -> String {
    format!("shiika_vtable_{}", classname.0)
}

/// Returns llvm function name of the given method
pub fn method_func_name(method_name: &MethodFullname) -> LlvmFuncName {
    LlvmFuncName(mangle_method(&method_name.full_name))
}

/// Returns llvm function name of the given constant
pub fn llvm_const_name(name: &ConstFullname) -> String {
    mangle_const(&name.0)
}
