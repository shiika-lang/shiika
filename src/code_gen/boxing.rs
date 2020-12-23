use crate::code_gen::*;
use crate::names::*;
use crate::ty;
use inkwell::values::*;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Generate llvm funcs about boxing
    pub fn gen_boxing_funcs(&self) {
        let fn_type = self.llvm_type(&ty::raw("Bool")).fn_type(&[self.i1_type.into()], false);
        self.module.add_function("box_bool", fn_type, None);
        let fn_type = self.i1_type.fn_type(&[self.llvm_type(&ty::raw("Bool")).into()], false);
        self.module.add_function("unbox_bool", fn_type, None);
        let fn_type = self.llvm_type(&ty::raw("Int")).fn_type(&[self.i64_type.into()], false);
        self.module.add_function("box_int", fn_type, None);
        let fn_type = self.i64_type.fn_type(&[self.llvm_type(&ty::raw("Int")).into()], false);
        self.module.add_function("unbox_int", fn_type, None);
        let fn_type = self.llvm_type(&ty::raw("Float")).fn_type(&[self.f64_type.into()], false);
        self.module.add_function("box_float", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.llvm_type(&ty::raw("Float")).into()], false);
        self.module.add_function("unbox_float", fn_type, None);
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
    }

    /// Convert LLVM bool(i1) into Shiika Bool
    pub fn box_bool<'a>(&'a self, b: inkwell::values::IntValue<'a>) -> inkwell::values::BasicValueEnum {
        let f = self.module.get_function("box_bool").unwrap();
        self.builder.build_call(f, &[b.into()], "bool").try_as_basic_value().left().unwrap()
    }

    /// Convert Shiika Bool into LLVM bool(i1)
    pub fn unbox_bool<'a>(
        &'a self,
        sk_bool: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::IntValue {
        let f = self.module.get_function("unbox_bool").unwrap();
        self.builder.build_call(f, &[sk_bool.into()], "b").try_as_basic_value().left().unwrap().into_int_value()
    }

    /// Convert LLVM int into Shiika Int
    pub fn box_int<'a>(&'a self, i: &inkwell::values::IntValue<'a>) -> inkwell::values::BasicValueEnum {
        let f = self.module.get_function("box_int").unwrap();
        self.builder.build_call(f, &[i.as_basic_value_enum().into()], "int").try_as_basic_value().left().unwrap()
    }

    /// Convert Shiika Int into LLVM int
    pub fn unbox_int<'a>(
        &'a self,
        sk_int: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::IntValue {
        let f = self.module.get_function("unbox_int").unwrap();
        self.builder.build_call(f, &[sk_int.into()], "i").try_as_basic_value().left().unwrap().into_int_value()
    }

    /// Convert LLVM float into Shiika Float
    pub fn box_float<'a>(
        &'a self,
        fl: &inkwell::values::FloatValue<'a>,
    ) -> inkwell::values::BasicValueEnum {
        let f = self.module.get_function("box_float").unwrap();
        self.builder.build_call(f, &[fl.as_basic_value_enum().into()], "float").try_as_basic_value().left().unwrap()
    }

    /// Convert Shiika Float into LLVM float
    pub fn unbox_float<'a>(
        &'a self,
        sk_float: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::FloatValue {
        let f = self.module.get_function("unbox_float").unwrap();
        self.builder.build_call(f, &[sk_float.into()], "f").try_as_basic_value().left().unwrap().into_float_value()
    }

    /// Convert LLVM i8* into Shiika::Internal::Ptr
    pub fn box_i8ptr<'a>(
        &'a self,
        p: inkwell::values::PointerValue,
    ) -> inkwell::values::BasicValueEnum {
        let sk_obj = self.allocate_sk_obj(&class_fullname("Shiika::Internal::Ptr"), "sk_ptr");
        self.build_ivar_store(&sk_obj, 0, p.as_basic_value_enum(), "@llvm_ptr");
        sk_obj
    }

    /// Convert Shiika::Internal::Ptr into LLVM i8*
    pub fn unbox_i8ptr<'a>(
        &'a self,
        sk_obj: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::PointerValue {
        self.build_ivar_load(sk_obj, 0, "@llvm_ptr")
            .into_pointer_value()
    }
}
