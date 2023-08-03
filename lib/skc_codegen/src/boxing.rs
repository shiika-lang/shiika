use crate::utils::llvm_func_name;
use crate::values::*;
use crate::CodeGen;
use inkwell::types::*;
use inkwell::values::BasicValue;
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
            .fn_type(&[self.llvm_type(&ty::raw("Bool")).into()], false);
        self.module.add_function("unbox_bool", fn_type, None);
        let fn_type = self
            .llvm_type(&ty::raw("Int"))
            .fn_type(&[self.i64_type.into()], false);
        self.module.add_function("box_int", fn_type, None);
        let fn_type = self
            .i64_type
            .fn_type(&[self.llvm_type(&ty::raw("Int")).into()], false);
        self.module.add_function("unbox_int", fn_type, None);
        let fn_type = self
            .llvm_type(&ty::raw("Float"))
            .fn_type(&[self.f64_type.into()], false);
        self.module.add_function("box_float", fn_type, None);
        let fn_type = self
            .f64_type
            .fn_type(&[self.llvm_type(&ty::raw("Float")).into()], false);
        self.module.add_function("unbox_float", fn_type, None);
        let fn_type = self
            .llvm_type(&ty::raw("Shiika::Internal::Ptr"))
            .fn_type(&[self.i8ptr_type.into()], false);
        self.module.add_function("box_i8ptr", fn_type, None);
        let fn_type = self.i8ptr_type.fn_type(
            &[self.llvm_type(&ty::raw("Shiika::Internal::Ptr")).into()],
            false,
        );
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
        sk_bool.ivar_store_raw(self, "@llvm_bool", i1_val);
        self.build_return(&sk_bool);

        // unbox_bool
        let function = self.module.get_function("unbox_bool").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_bool = SkObj::new(ty::raw("Bool"), function.get_params()[0]);
        let i1_val = self.build_ivar_load_raw(sk_bool, "@llvm_bool");
        self.builder.build_return(Some(&i1_val));

        // box_int
        let function = self.module.get_function("box_int").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let i64_val = function.get_params()[0];
        let sk_int = self.allocate_sk_obj(&class_fullname("Int"), "sk_int");
        sk_int.ivar_store_raw(self, "@llvm_int", i64_val);
        self.build_return(&sk_int);

        // unbox_int
        let function = self.module.get_function("unbox_int").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_int = SkObj::new(
            ty::raw("Int"),
            function.get_params()[0].into_pointer_value(),
        );
        let i64_val = self.build_ivar_load_raw(sk_int, "@llvm_int");
        self.builder.build_return(Some(&i64_val));

        // box_float
        let function = self.module.get_function("box_float").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let f64_val = function.get_params()[0];
        let sk_float = self.allocate_sk_obj(&class_fullname("Float"), "sk_float");
        sk_float.ivar_store_raw(self, "@llvm_float", f64_val);
        self.build_return(&sk_float);

        // unbox_float
        let function = self.module.get_function("unbox_float").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_float = SkObj::new(
            ty::raw("Float"),
            function.get_params()[0].into_pointer_value(),
        );
        let f64_val = self.build_ivar_load_raw(sk_float, "@llvm_float");
        self.builder.build_return(Some(&f64_val));

        // box_i8ptr
        let function = self.module.get_function("box_i8ptr").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let i8ptr = function.get_params()[0];
        let sk_ptr = self.allocate_sk_obj(&class_fullname("Shiika::Internal::Ptr"), "sk_ptr");
        sk_ptr.ivar_store_raw(self, "@llvm_i8ptr", i8ptr);
        self.build_return(&sk_ptr);

        // unbox_i8ptr
        let function = self.module.get_function("unbox_i8ptr").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let sk_ptr = SkObj::new(
            ty::raw("Shiika::Internal::Ptr"),
            function.get_params()[0].into_pointer_value(),
        );
        let i8ptr = self.build_ivar_load_raw(sk_ptr, "@llvm_i8ptr");
        self.builder.build_return(Some(&i8ptr));

        // gen_literal_string
        self.impl_gen_literal_string();
    }

    fn impl_gen_literal_string(&self) {
        let function = self.module.get_function("gen_literal_string").unwrap();
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let receiver = self.gen_const_ref(&toplevel_const("String"), &ty::meta("String"));
        let str_i8ptr = function.get_nth_param(0).unwrap();
        let bytesize = function.get_nth_param(1).unwrap().into_int_value();
        let args = vec![self.box_i8ptr(str_i8ptr), self.box_int(&bytesize)];
        let sk_str = self.call_method_func(
            &method_fullname(metaclass_fullname("String").into(), "new"),
            receiver,
            &args,
            ty::raw("String"),
            "sk_str",
        );
        self.build_return(&sk_str);
    }

    /// Convert LLVM bool(i1) into Shiika Bool
    pub fn box_bool(&self, b: inkwell::values::IntValue<'run>) -> SkObj<'run> {
        SkObj::new(
            ty::raw("Bool"),
            self.call_llvm_func(&llvm_func_name("box_bool"), &[b.into()], "sk_bool")
                .into_pointer_value(),
        )
    }

    /// Convert Shiika Bool into LLVM bool(i1)
    pub fn unbox_bool(&self, sk_bool: SkObj<'run>) -> inkwell::values::IntValue<'run> {
        self.call_llvm_func(
            &llvm_func_name("unbox_bool"),
            &[sk_bool.0.into()],
            "llvm_bool",
        )
        .into_int_value()
    }

    /// Convert LLVM int into Shiika Int
    pub fn box_int(&self, i: &inkwell::values::IntValue<'run>) -> SkObj<'run> {
        SkObj::new(
            ty::raw("Int"),
            self.call_llvm_func(
                &llvm_func_name("box_int"),
                &[i.as_basic_value_enum().into()],
                "sk_int",
            )
            .into_pointer_value(),
        )
    }

    /// Convert Shiika Int into LLVM int
    pub fn unbox_int(&self, sk_int: SkObj<'run>) -> inkwell::values::IntValue<'run> {
        self.call_llvm_func(
            &llvm_func_name("unbox_int"),
            &[sk_int.0.as_basic_value_enum().into()],
            "llvm_int",
        )
        .into_int_value()
    }

    /// Convert LLVM float into Shiika Float
    pub fn box_float(&self, fl: &inkwell::values::FloatValue<'run>) -> SkObj<'run> {
        SkObj::new(
            ty::raw("Float"),
            self.call_llvm_func(
                &llvm_func_name("box_float"),
                &[fl.as_basic_value_enum().into()],
                "sk_float",
            )
            .into_pointer_value(),
        )
    }

    /// Convert Shiika Float into LLVM float
    pub fn unbox_float(&self, sk_float: SkObj<'run>) -> inkwell::values::FloatValue<'run> {
        self.call_llvm_func(
            &llvm_func_name("unbox_float"),
            &[sk_float.0.into()],
            "llvm_float",
        )
        .into_float_value()
    }

    /// Convert LLVM i8* into Shiika::Internal::Ptr
    pub fn box_i8ptr(&self, p: inkwell::values::BasicValueEnum<'run>) -> SkObj<'run> {
        SkObj::new(
            ty::raw("Shiika::Internal::Ptr"),
            self.call_llvm_func(&llvm_func_name("box_i8ptr"), &[p.into()], "sk_ptr")
                .into_pointer_value(),
        )
    }

    /// Convert Shiika::Internal::Ptr into LLVM i8*
    pub fn unbox_i8ptr(&self, sk_obj: SkObj<'run>) -> I8Ptr<'run> {
        I8Ptr(
            self.call_llvm_func(
                &llvm_func_name("unbox_i8ptr"),
                &[sk_obj.0.into()],
                "llvm_ptr",
            )
            .into_pointer_value(),
        )
    }
}
