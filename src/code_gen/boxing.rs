use crate::code_gen::*;
use crate::names::*;
use crate::ty;
use inkwell::values::*;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Generate llvm funcs about boxing
    pub fn gen_boxing_funcs(&self) {
        let fn_type = self
            .llvm_type(&ty::raw("Bool"))
            .fn_type(&[self.i1_type.into()], false);
        self.module.add_function("box_bool", fn_type, None);
        let fn_type = self
            .i1_type
            .fn_type(&[self.llvm_type(&ty::raw("Bool"))], false);
        self.module.add_function("unbox_bool", fn_type, None);
        let fn_type = self
            .llvm_type(&ty::raw("Int"))
            .fn_type(&[self.i64_type.into()], false);
        self.module.add_function("box_int", fn_type, None);
        let fn_type = self
            .i64_type
            .fn_type(&[self.llvm_type(&ty::raw("Int"))], false);
        self.module.add_function("unbox_int", fn_type, None);
        let fn_type = self
            .llvm_type(&ty::raw("Float"))
            .fn_type(&[self.f64_type.into()], false);
        self.module.add_function("box_float", fn_type, None);
        let fn_type = self
            .f64_type
            .fn_type(&[self.llvm_type(&ty::raw("Float"))], false);
        self.module.add_function("unbox_float", fn_type, None);
        let fn_type = self
            .llvm_type(&ty::raw("Shiika::Internal::Ptr"))
            .fn_type(&[self.i8ptr_type.into()], false);
        self.module.add_function("box_i8ptr", fn_type, None);
        let fn_type = self
            .i8ptr_type
            .fn_type(&[self.llvm_type(&ty::raw("Shiika::Internal::Ptr"))], false);
        self.module.add_function("unbox_i8ptr", fn_type, None);
        let fn_type = self
            .llvm_type(&ty::raw("String"))
            .fn_type(&[self.i8ptr_type.into(), self.i64_type.into()], false);
        self.module
            .add_function("gen_literal_string", fn_type, None);
    }

    /// Generate body of llvm funcs about boxing
    pub fn impl_boxing_funcs(&self) {
        // box_bool
        let function = self.module.get_function("box_bool").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let i1_val = function.get_params()[0];
        let sk_bool = self.allocate_sk_obj(&class_fullname("Bool"), "sk_bool");
        self.build_ivar_store(&sk_bool, 0, i1_val.as_basic_value_enum(), "@llvm_bool");
        self.builder.build_return(Some(&sk_bool));

        // unbox_bool
        let function = self.module.get_function("unbox_bool").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_bool = function.get_params()[0];
        let i1_val = self.build_ivar_load(sk_bool, 0, "@llvm_bool");
        self.builder.build_return(Some(&i1_val));

        // box_int
        let function = self.module.get_function("box_int").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let i64_val = function.get_params()[0];
        let sk_int = self.allocate_sk_obj(&class_fullname("Int"), "sk_int");
        self.build_ivar_store(&sk_int, 0, i64_val.as_basic_value_enum(), "@llvm_int");
        self.builder.build_return(Some(&sk_int));

        // unbox_int
        let function = self.module.get_function("unbox_int").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_int = function.get_params()[0];
        let i64_val = self.build_ivar_load(sk_int, 0, "@llvm_int");
        self.builder.build_return(Some(&i64_val));

        // box_float
        let function = self.module.get_function("box_float").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let f64_val = function.get_params()[0];
        let sk_float = self.allocate_sk_obj(&class_fullname("Float"), "sk_float");
        self.build_ivar_store(&sk_float, 0, f64_val.as_basic_value_enum(), "@llvm_float");
        self.builder.build_return(Some(&sk_float));

        // unbox_float
        let function = self.module.get_function("unbox_float").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_float = function.get_params()[0];
        let f64_val = self.build_ivar_load(sk_float, 0, "@llvm_float");
        self.builder.build_return(Some(&f64_val));

        // box_i8ptr
        let function = self.module.get_function("box_i8ptr").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let i8ptr = function.get_params()[0];
        let sk_ptr = self.allocate_sk_obj(&class_fullname("Shiika::Internal::Ptr"), "sk_ptr");
        self.build_ivar_store(&sk_ptr, 0, i8ptr.as_basic_value_enum(), "@llvm_i8ptr");
        self.builder.build_return(Some(&sk_ptr));

        // unbox_i8ptr
        let function = self.module.get_function("unbox_i8ptr").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_ptr = function.get_params()[0];
        let i8ptr = self.build_ivar_load(sk_ptr, 0, "@llvm_i8ptr");
        self.builder.build_return(Some(&i8ptr));

        // gen_literal_string
        self.impl_gen_literal_string();
    }

    fn impl_gen_literal_string(&self) {
        let function = self.module.get_function("gen_literal_string").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let func = self.get_llvm_func("Meta:String#new");
        let receiver_value = self.gen_const_ref(&toplevel_const("String"));
        let str_i8ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let bytesize = function.get_nth_param(1).unwrap().into_int_value();
        let arg_values = vec![self.box_i8ptr(str_i8ptr), self.box_int(&bytesize)];
        let sk_str = self.gen_llvm_function_call(func, receiver_value, arg_values);
        self.builder.build_return(Some(&sk_str));
    }

    /// Convert LLVM bool(i1) into Shiika Bool
    pub fn box_bool(
        &self,
        b: inkwell::values::IntValue<'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let f = self.module.get_function("box_bool").unwrap();
        self.builder
            .build_call(f, &[b.into()], "bool")
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    /// Convert Shiika Bool into LLVM bool(i1)
    pub fn unbox_bool(
        &self,
        sk_bool: inkwell::values::BasicValueEnum<'run>,
    ) -> inkwell::values::IntValue<'run> {
        let f = self.module.get_function("unbox_bool").unwrap();
        self.builder
            .build_call(f, &[sk_bool], "b")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    /// Convert LLVM int into Shiika Int
    pub fn box_int(
        &self,
        i: &inkwell::values::IntValue<'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let f = self.module.get_function("box_int").unwrap();
        self.builder
            .build_call(f, &[i.as_basic_value_enum()], "int")
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    /// Convert Shiika Int into LLVM int
    pub fn unbox_int(
        &self,
        sk_int: inkwell::values::BasicValueEnum<'run>,
    ) -> inkwell::values::IntValue<'run> {
        let f = self.module.get_function("unbox_int").unwrap();
        self.builder
            .build_call(f, &[sk_int], "i")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    /// Convert LLVM float into Shiika Float
    pub fn box_float(
        &self,
        fl: &inkwell::values::FloatValue<'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let f = self.module.get_function("box_float").unwrap();
        self.builder
            .build_call(f, &[fl.as_basic_value_enum()], "float")
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    /// Convert Shiika Float into LLVM float
    pub fn unbox_float(
        &self,
        sk_float: inkwell::values::BasicValueEnum<'run>,
    ) -> inkwell::values::FloatValue<'run> {
        let f = self.module.get_function("unbox_float").unwrap();
        self.builder
            .build_call(f, &[sk_float], "f")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_float_value()
    }

    /// Convert LLVM i8* into Shiika::Internal::Ptr
    pub fn box_i8ptr(
        &self,
        p: inkwell::values::PointerValue<'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let f = self.module.get_function("box_i8ptr").unwrap();
        self.builder
            .build_call(f, &[p.as_basic_value_enum()], "sk_ptr")
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    /// Convert Shiika::Internal::Ptr into LLVM i8*
    pub fn unbox_i8ptr(
        &self,
        sk_obj: inkwell::values::BasicValueEnum<'run>,
    ) -> inkwell::values::PointerValue<'run> {
        let f = self.module.get_function("unbox_i8ptr").unwrap();
        self.builder
            .build_call(f, &[sk_obj], "p")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }
}
