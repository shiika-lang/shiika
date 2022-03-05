use crate::utils::llvm_func_name;
use crate::values::*;
use crate::CodeGen;
use inkwell::types::*;
use inkwell::values::*;
use shiika_core::{names::*, ty};

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

        let i1_val = SkObj(function.get_params()[0]);
        let sk_bool = self.allocate_sk_obj(&module_fullname("Bool"), "sk_bool");
        self.build_ivar_store(&sk_bool, 0, i1_val, "@llvm_bool");
        self.build_return(&sk_bool);

        // unbox_bool
        let function = self.module.get_function("unbox_bool").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_bool = SkObj(function.get_params()[0]);
        let i1_val = self.build_ivar_load(sk_bool, 0, "@llvm_bool");
        self.build_return(&i1_val);

        // box_int
        let function = self.module.get_function("box_int").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let i64_val = SkObj(function.get_params()[0]);
        let sk_int = self.allocate_sk_obj(&module_fullname("Int"), "sk_int");
        self.build_ivar_store(&sk_int, 0, i64_val, "@llvm_int");
        self.build_return(&sk_int);

        // unbox_int
        let function = self.module.get_function("unbox_int").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_int = SkObj(function.get_params()[0]);
        let i64_val = self.build_ivar_load(sk_int, 0, "@llvm_int");
        self.build_return(&i64_val);

        // box_float
        let function = self.module.get_function("box_float").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let f64_val = SkObj(function.get_params()[0]);
        let sk_float = self.allocate_sk_obj(&module_fullname("Float"), "sk_float");
        self.build_ivar_store(&sk_float, 0, f64_val, "@llvm_float");
        self.build_return(&sk_float);

        // unbox_float
        let function = self.module.get_function("unbox_float").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_float = SkObj(function.get_params()[0]);
        let f64_val = self.build_ivar_load(sk_float, 0, "@llvm_float");
        self.build_return(&f64_val);

        // box_i8ptr
        let function = self.module.get_function("box_i8ptr").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let i8ptr = function.get_params()[0];
        let sk_ptr = self.allocate_sk_obj(&module_fullname("Shiika::Internal::Ptr"), "sk_ptr");
        self.build_ivar_store_raw(&sk_ptr, 0, i8ptr, "@llvm_i8ptr");
        self.build_return(&sk_ptr);

        // unbox_i8ptr
        let function = self.module.get_function("unbox_i8ptr").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_ptr = SkObj(function.get_params()[0]);
        let i8ptr = self.build_ivar_load(sk_ptr, 0, "@llvm_i8ptr");
        self.build_return(&i8ptr);

        // gen_literal_string
        self.impl_gen_literal_string();
    }

    fn impl_gen_literal_string(&self) {
        let function = self.module.get_function("gen_literal_string").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let receiver = self.gen_const_ref(&toplevel_const("String"));
        let str_i8ptr = function.get_nth_param(0).unwrap();
        let bytesize = function.get_nth_param(1).unwrap().into_int_value();
        let args = vec![self.box_i8ptr(str_i8ptr), self.box_int(&bytesize)];
        let sk_str = self.call_method_func(
            &method_fullname(&metamodule_fullname("String"), "new"),
            receiver,
            &args,
            "sk_str",
        );
        self.build_return(&sk_str);
    }

    /// Convert LLVM bool(i1) into Shiika Bool
    pub fn box_bool(&self, b: inkwell::values::IntValue<'run>) -> SkObj<'run> {
        SkObj(self.call_llvm_func(&llvm_func_name("box_bool"), &[b.into()], "sk_bool"))
    }

    /// Convert Shiika Bool into LLVM bool(i1)
    pub fn unbox_bool(&self, sk_bool: SkObj<'run>) -> inkwell::values::IntValue<'run> {
        self.call_llvm_func(&llvm_func_name("unbox_bool"), &[sk_bool.0], "llvm_bool")
            .into_int_value()
    }

    /// Convert LLVM int into Shiika Int
    pub fn box_int(&self, i: &inkwell::values::IntValue<'run>) -> SkObj<'run> {
        SkObj(self.call_llvm_func(
            &llvm_func_name("box_int"),
            &[i.as_basic_value_enum()],
            "sk_int",
        ))
    }

    /// Convert Shiika Int into LLVM int
    pub fn unbox_int(&self, sk_int: SkObj<'run>) -> inkwell::values::IntValue<'run> {
        self.call_llvm_func(&llvm_func_name("unbox_int"), &[sk_int.0], "llvm_int")
            .into_int_value()
    }

    /// Convert LLVM float into Shiika Float
    pub fn box_float(&self, fl: &inkwell::values::FloatValue<'run>) -> SkObj<'run> {
        SkObj(self.call_llvm_func(
            &llvm_func_name("box_float"),
            &[fl.as_basic_value_enum()],
            "sk_float",
        ))
    }

    /// Convert Shiika Float into LLVM float
    pub fn unbox_float(&self, sk_float: SkObj<'run>) -> inkwell::values::FloatValue<'run> {
        self.call_llvm_func(&llvm_func_name("unbox_float"), &[sk_float.0], "llvm_float")
            .into_float_value()
    }

    /// Convert LLVM i8* into Shiika::Internal::Ptr
    pub fn box_i8ptr(&self, p: inkwell::values::BasicValueEnum<'run>) -> SkObj<'run> {
        SkObj(self.call_llvm_func(&llvm_func_name("box_i8ptr"), &[p], "sk_ptr"))
    }

    /// Convert Shiika::Internal::Ptr into LLVM i8*
    pub fn unbox_i8ptr(&self, sk_obj: SkObj<'run>) -> I8Ptr<'run> {
        I8Ptr(
            self.call_llvm_func(&llvm_func_name("unbox_i8ptr"), &[sk_obj.0], "llvm_ptr")
                .into_pointer_value(),
        )
    }
}
