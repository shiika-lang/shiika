mod code_gen_context;
mod gen_exprs;
use std::collections::HashMap;
use inkwell::AddressSpace;
use inkwell::values::*;
use inkwell::types::*;
use crate::error::Error;
use crate::ty::*;
use crate::hir::*;
use crate::names::*;
use crate::code_gen::code_gen_context::*;

pub struct CodeGen<'hir> {
    pub context: inkwell::context::Context,
    pub module: inkwell::module::Module,
    pub builder: inkwell::builder::Builder,
    pub i1_type: inkwell::types::IntType,
    pub i8_type: inkwell::types::IntType,
    pub i8ptr_type: inkwell::types::PointerType,
    pub i32_type: inkwell::types::IntType,
    pub i64_type: inkwell::types::IntType,
    pub f64_type: inkwell::types::FloatType,
    pub void_type: inkwell::types::VoidType,
    pub llvm_struct_types: HashMap<ClassFullname, inkwell::types::StructType>,
    str_literals: &'hir Vec<String>,
    /// Toplevel `self`
    the_main: Option<inkwell::values::BasicValueEnum>,
}

impl<'hir> CodeGen<'hir> {
    pub fn new(hir: &'hir Hir) -> CodeGen<'hir> {
        let context = inkwell::context::Context::create();
        let module = context.create_module("main");
        let builder = context.create_builder();
        CodeGen {
            context,
            module,
            builder,
            i1_type: inkwell::types::IntType::bool_type(),
            i8_type: inkwell::types::IntType::i8_type(),
            i8ptr_type: inkwell::types::IntType::i8_type().ptr_type(AddressSpace::Generic),
            i32_type: inkwell::types::IntType::i32_type(),
            i64_type: inkwell::types::IntType::i64_type(),
            f64_type: inkwell::types::FloatType::f64_type(),
            void_type: inkwell::types::VoidType::void_type(),
            llvm_struct_types: HashMap::new(),
            str_literals: &hir.str_literals,
            the_main: None,
        }
    }

    pub fn gen_program(&mut self, hir: &Hir) -> Result<(), Error> {
        self.gen_declares();
        self.gen_class_structs(&hir.sk_classes);
        self.gen_string_literals(&hir.str_literals);
        self.gen_constant_ptrs(&hir.constants);
        self.gen_method_funcs(&hir.sk_methods);
        self.gen_methods(&hir.sk_methods)?;
        self.gen_const_inits(&hir.const_inits)?;
        self.gen_user_main(&hir.main_exprs)?;
        self.gen_main()?;
        Ok(())
    }

    fn gen_declares(&self) {
        let fn_type = self.i32_type.fn_type(&[self.i32_type.into()], false);
        self.module.add_function("putchar", fn_type, None);
        let fn_type = self.i32_type.fn_type(&[self.i8ptr_type.into()], true);
        self.module.add_function("printf", fn_type, None);
        let fn_type = self.i32_type.fn_type(&[self.i8ptr_type.into()], false);
        self.module.add_function("puts", fn_type, None);

        let fn_type = self.void_type.fn_type(&[], false);
        self.module.add_function("GC_init", fn_type, None);
        let fn_type = self.i8ptr_type.fn_type(&[IntType::i64_type().into()], false);
        self.module.add_function("GC_malloc", fn_type, None);
        let fn_type = self.i8ptr_type.fn_type(&[self.i8ptr_type.into(), IntType::i64_type().into()], false);
        self.module.add_function("GC_realloc", fn_type, None);
        let fn_type = self.void_type.fn_type(&[self.i8ptr_type.into(), self.i8ptr_type.into(), IntType::i64_type().into(),
                                               self.i32_type.into(), self.i1_type.into()], false);
        self.module.add_function("llvm.memcpy.p0i8.p0i8.i64", fn_type, None);

        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("sin", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("cos", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("sqrt", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("fabs", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("floor", fn_type, None);

        let str_type = self.i8_type.array_type(3);
        let global = self.module.add_global(str_type, None, "putd_tmpl");
        global.set_linkage(inkwell::module::Linkage::Internal);
        global.set_initializer(&self.i8_type.const_array(&[self.i8_type.const_int(37, false), // %
                                                           self.i8_type.const_int(100, false), // d
                                                           self.i8_type.const_int(  0, false)]));
        global.set_constant(true);

        let str_type = self.i8_type.array_type(3);
        let global = self.module.add_global(str_type, None, "putf_tmpl");
        global.set_linkage(inkwell::module::Linkage::Internal);
        global.set_initializer(&self.i8_type.const_array(&[self.i8_type.const_int(37, false), // %
                                                           self.i8_type.const_int(102, false), // f
                                                           self.i8_type.const_int(  0, false)]));
        global.set_constant(true);
    }

    fn gen_user_main(&mut self, main_exprs: &HirExpressions) -> Result<(), Error> {
        // define void @user_main()
        let user_main_type = self.void_type.fn_type(&[], false);
        let function = self.module.add_function("user_main", user_main_type, None);
        let create_main_block = self.context.append_basic_block(&function, "CreateMain");
        let user_main_block = self.context.append_basic_block(&function, "UserMain");

        // CreateMain:
        self.builder.position_at_end(&create_main_block);
        self.the_main = Some(self.allocate_sk_obj(&class_fullname("Object"), "main"));
        self.builder.build_unconditional_branch(&user_main_block);

        // UserMain:
        self.builder.position_at_end(&user_main_block);
        let mut ctx = CodeGenContext::new(function);
        self.gen_exprs(&mut ctx, &main_exprs)?;
        self.builder.build_return(None);
        Ok(())
    }

    fn gen_main(&mut self) -> Result<(), Error> {
        // define i32 @main() {
        let main_type = self.i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", main_type, None);
        let basic_block = self.context.append_basic_block(&function, "");
        self.builder.position_at_end(&basic_block);

        // Call GC_init
        let func = self.module.get_function("GC_init").unwrap();
        self.builder.build_call(func, &[], "");

        // Call init_constants, user_main
        let func = self.module.get_function("init_constants").unwrap();
        self.builder.build_call(func, &[], "");
        let func = self.module.get_function("user_main").unwrap();
        self.builder.build_call(func, &[], "");

        // ret i32 0
        self.builder.build_return(Some(&self.i32_type.const_int(0, false)));
        Ok(())
    }

    /// Create llvm struct types for Shiika objects
    fn gen_class_structs(&mut self, classes: &HashMap<ClassFullname, SkClass>) {
        // 1. Create struct type for each class
        for name in classes.keys() {
            self.llvm_struct_types.insert(
                name.clone(),
                self.context.opaque_struct_type(&name.0)
            );
        }

        // 2. Set ivars
        for (name, sk_class) in classes {
            let struct_type = self.llvm_struct_types.get(&name).unwrap();
            struct_type.set_body(&self.llvm_field_types(&sk_class.ivars), false);
        }
    }

    fn llvm_field_types(&self, ivars: &HashMap<String, SkIVar>) -> Vec<inkwell::types::BasicTypeEnum>
    {
        let mut values = ivars.values().collect::<Vec<_>>();
        values.sort_by_key(|ivar| ivar.idx);
        values.iter().map(|ivar| {
            self.llvm_type(&ivar.ty)
        }).collect::<Vec<_>>()
    }

    /// Generate llvm constants for string literals
    fn gen_string_literals(&self, str_literals: &[String]) {
        str_literals.iter().enumerate().for_each(|(i, s)| {
            // PERF: how to avoid .to_string?
            let s_with_null = s.to_string() + "\0";
            let bytesize = s_with_null.len();
            let str_type = self.i8_type.array_type(bytesize as u32);
            let global = self.module.add_global(str_type, None, &format!("str_{}", i));
            global.set_linkage(inkwell::module::Linkage::Internal);
            let content = s_with_null.into_bytes().iter().map(|byte| {
                self.i8_type.const_int((*byte).into(), false)
            }).collect::<Vec<_>>();
            global.set_initializer(&self.i8_type.const_array(&content))
        })
    }

    fn gen_constant_ptrs(&self, constants: &HashMap<ConstFullname, TermTy>) {
        for (fullname, ty) in constants {
            let name = &fullname.0;
            let global = self.module.add_global(self.llvm_type(&ty), None, name);
            global.set_linkage(inkwell::module::Linkage::Internal);
            let null = self.i32_type.ptr_type(AddressSpace::Generic).const_null();
            match self.llvm_zero_value(ty) {
                Some(zero) => global.set_initializer(&zero),
                None       => global.set_initializer(&null),
            }
        }
    }

    fn gen_const_inits(&self, const_inits: &[HirExpression]) -> Result<(), Error> {
        // define void @init_constants()
        let fn_type = self.void_type.fn_type(&[], false);
        let function = self.module.add_function("init_constants", fn_type, None);
        let basic_block = self.context.append_basic_block(&function, "");
        self.builder.position_at_end(&basic_block);

        let mut ctx = CodeGenContext::new(function);
        for expr in const_inits {
            self.gen_expr(&mut ctx, &expr)?;
        }

        // Generate void
        let ptr = self.module.get_global(&"::void").unwrap().as_pointer_value();
        let value = self.allocate_sk_obj(&class_fullname("Void"), "void_obj");
        self.builder.build_store(ptr, value);

        self.builder.build_return(None);
        Ok(())
    }

    /// Create inkwell functions
    fn gen_method_funcs(&self,
                        methods: &HashMap<ClassFullname, Vec<SkMethod>>) {
        methods.iter().for_each(|(cname, sk_methods)| {
            sk_methods.iter().for_each(|method| {
                let self_ty = cname.to_ty();
                let func_type = self.llvm_func_type(&self_ty, &method.signature);
                self.module.add_function(&method.signature.fullname.full_name, func_type, None);
            })
        })
    }

    fn gen_methods(&self, methods: &HashMap<ClassFullname, Vec<SkMethod>>) -> Result<(), Error> {
        methods.values().try_for_each(|sk_methods| {
            sk_methods.iter().try_for_each(|method|
                self.gen_method(&method)
            )
        })
    }

    fn gen_method(&self, method: &SkMethod) -> Result<(), Error> {
        // LLVM function
        let function = self.module.get_function(&method.signature.fullname.full_name)
            .unwrap_or_else(|| panic!("[BUG] get_function not found: {:?}", method.signature));

        // Set param names
        for (i, param) in function.get_param_iter().enumerate() {
            if i == 0 {
                inkwell_set_name(param, "self")
            }
            else {
                inkwell_set_name(param, &method.signature.params[i-1].name)
            }
        }

        // Main basic block
        let basic_block = self.context.append_basic_block(&function, "");
        self.builder.position_at_end(&basic_block);

        // Method body
        match &method.body {
            SkMethodBody::RustMethodBody { gen } => {
                gen(self, &function)?
            },
            SkMethodBody::RustClosureMethodBody { boxed_gen } => {
                boxed_gen(self, &function)?
            },
            SkMethodBody::ShiikaMethodBody { exprs }=> {
                self.gen_shiika_method_body(function,
                                            method.signature.ret_ty.is_void_type(),
                                            &exprs)?
            }
        }
        Ok(())
    }

    fn gen_shiika_method_body(&self,
                              function: inkwell::values::FunctionValue,
                              void_method: bool,
                              exprs: &HirExpressions) -> Result<(), Error> {
        let mut ctx = CodeGenContext::new(function);
        let last_value = self.gen_exprs(&mut ctx, exprs)?;
        if void_method {
            self.builder.build_return(None);
        }
        else {
            self.builder.build_return(Some(&last_value));
        }
        Ok(())
    }
}

// Question: is there a better way to do this?
fn inkwell_set_name(val: BasicValueEnum, name: &str) {
    match val {
        BasicValueEnum::ArrayValue(v) => v.set_name(name),
        BasicValueEnum::IntValue(v) => v.set_name(name),
        BasicValueEnum::FloatValue(v) => v.set_name(name),
        BasicValueEnum::PointerValue(v) => v.set_name(name),
        BasicValueEnum::StructValue(v) => v.set_name(name),
        BasicValueEnum::VectorValue(v) => v.set_name(name),
    }
}
