use crate::code_gen::*;
use crate::names::*;
use inkwell::values::*;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Convert LLVM bool(i1) into Shiika Bool
    pub fn box_bool<'a>(&'a self, b: inkwell::values::IntValue) -> inkwell::values::BasicValueEnum {
        let sk_bool = self.allocate_sk_obj(&class_fullname("Bool"), "bool");
        self.build_ivar_store(&sk_bool, 0, b.as_basic_value_enum(), "bool");
        sk_bool
    }

    /// Convert Shiika Bool into LLVM bool(i1)
    pub fn unbox_bool<'a>(
        &'a self,
        sk_bool: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::IntValue {
        self.build_ivar_load(sk_bool, 0, "bool").into_int_value()
    }

    /// Convert LLVM int into Shiika Int
    pub fn box_int(&self, int: &inkwell::values::IntValue) -> inkwell::values::BasicValueEnum {
        let sk_int = self.allocate_sk_obj(&class_fullname("Int"), "int");
        self.build_ivar_store(&sk_int, 0, int.as_basic_value_enum(), "int");
        sk_int
    }

    /// Convert Shiika Int into LLVM int
    pub fn unbox_int<'a>(
        &'a self,
        sk_int: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::IntValue {
        self.build_ivar_load(sk_int, 0, "int").into_int_value()
    }

    /// Convert LLVM float into Shiika Float
    pub fn box_float(
        &self,
        float: &inkwell::values::FloatValue,
    ) -> inkwell::values::BasicValueEnum {
        let sk_float = self.allocate_sk_obj(&class_fullname("Float"), "float");
        self.build_ivar_store(&sk_float, 0, float.as_basic_value_enum(), "float");
        sk_float
    }

    /// Convert Shiika Float into LLVM float
    pub fn unbox_float<'a>(
        &'a self,
        sk_float: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::FloatValue {
        self.build_ivar_load(sk_float, 0, "float")
            .into_float_value()
    }
}
