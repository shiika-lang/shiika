use inkwell::values::*;
use crate::code_gen::CodeGen;
use crate::names::*;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Convert LLVM float into Shiika Float
    pub fn box_float(&self, float: &inkwell::values::FloatValue) -> inkwell::values::BasicValueEnum {
        let sk_float = self.allocate_sk_obj(&class_fullname("Float"), "float");
        let ptr = self.builder.build_struct_gep(
            sk_float.into_pointer_value(),
            0,
            &"float_content"
        ).unwrap();
        self.builder.build_store(ptr, float.as_basic_value_enum());
        sk_float
    }

    /// Convert Shiika Float into LLVM float
    pub fn unbox_float<'a>(&'a self, sk_float: &'a inkwell::values::BasicValueEnum)
                      -> inkwell::values::FloatValue {
        let ptr = self.builder.build_struct_gep(
            sk_float.into_pointer_value(),
            0,
            &"float_content"
        ).unwrap();
        self.builder.build_load(ptr, "float_value").into_float_value()
    }
}
