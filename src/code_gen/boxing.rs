use crate::code_gen::*;
use crate::names::*;
use inkwell::values::*;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Convert LLVM bool(i1) into Shiika Bool
    pub fn box_bool<'a>(&'a self, b: &'a inkwell::values::IntValue) -> inkwell::values::IntValue {
        // 0b? -> 0b?10
        let b = self.builder.build_int_z_extend(*b, self.i64_type, "");
        let b = self
            .builder
            .build_left_shift(b, self.i64_type.const_int(2, false), "");
        self.builder
            .build_or(b, self.i64_type.const_int(0b10, false), "sk_bool")
    }

    /// Convert Shiika Bool into LLVM bool(i1)
    pub fn unbox_bool<'a>(
        &'a self,
        b: &'a inkwell::values::BasicValueEnum,
    ) -> inkwell::values::IntValue {
        // 0b? <- 0b?10
        let two = self.i64_type.const_int(2, false);
        let b = self
            .builder
            .build_right_shift(b.into_int_value(), two, false, "");
        self.builder
            .build_int_truncate(b, self.i1_type, "llvm_bool")
    }

    pub fn invert_sk_bool<'a>(
        &'a self,
        b: inkwell::values::BasicValueEnum<'a>,
    ) -> inkwell::values::IntValue {
        // 0b010 <-> 0b110
        let x = self.i64_type.const_int(0b100, false);
        self.builder
            .build_xor(b.into_int_value(), x, "invert_sk_bool")
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
        self.build_ivar_load(sk_float, 0, "float").into_float_value()
    }
}
