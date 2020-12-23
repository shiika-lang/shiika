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
    pub fn box_int(&self, int: &inkwell::values::IntValue) -> inkwell::values::BasicValueEnum {
        let sk_int = self.allocate_sk_obj(&class_fullname("Int"), "sk_int");
        self.build_ivar_store(&sk_int, 0, int.as_basic_value_enum(), "@llvm_int");
        sk_int
    }

    /// Convert Shiika Int into LLVM int
    pub fn unbox_int<'a>(
        &'a self,
        sk_int: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::IntValue {
        self.build_ivar_load(sk_int, 0, "@llvm_int")
            .into_int_value()
    }

    /// Convert LLVM float into Shiika Float
    pub fn box_float(
        &self,
        float: &inkwell::values::FloatValue,
    ) -> inkwell::values::BasicValueEnum {
        let sk_float = self.allocate_sk_obj(&class_fullname("Float"), "sk_float");
        self.build_ivar_store(&sk_float, 0, float.as_basic_value_enum(), "@llvm_float");
        sk_float
    }

    /// Convert Shiika Float into LLVM float
    pub fn unbox_float<'a>(
        &'a self,
        sk_float: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::FloatValue {
        self.build_ivar_load(sk_float, 0, "@llvm_float")
            .into_float_value()
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
