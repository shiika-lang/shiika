mod boxing;
mod code_gen_context;
mod gen_exprs;
mod lambda;
mod utils;
use crate::code_gen::code_gen_context::*;
use crate::code_gen::utils::llvm_vtable_name;
use crate::error::Error;
use crate::hir::*;
use crate::mir;
use crate::mir::*;
use crate::names::*;
use crate::ty::*;
use either::*;
use inkwell::types::*;
use inkwell::values::*;
use inkwell::AddressSpace;
use std::collections::HashMap;

/// CodeGen
///
/// 'hir > 'ictx >= 'run
///
/// 'hir: the Hir
/// 'ictx: inkwell context
/// 'run: code_gen::run()
pub struct CodeGen<'hir: 'ictx, 'run, 'ictx: 'run> {
    pub context: &'ictx inkwell::context::Context,
    pub module: &'run inkwell::module::Module<'ictx>,
    pub builder: &'run inkwell::builder::Builder<'ictx>,
    pub i1_type: inkwell::types::IntType<'ictx>,
    pub i8_type: inkwell::types::IntType<'ictx>,
    pub i8ptr_type: inkwell::types::PointerType<'ictx>,
    pub i32_type: inkwell::types::IntType<'ictx>,
    pub i64_type: inkwell::types::IntType<'ictx>,
    pub f64_type: inkwell::types::FloatType<'ictx>,
    pub void_type: inkwell::types::VoidType<'ictx>,
    pub llvm_struct_types: HashMap<ClassFullname, inkwell::types::StructType<'ictx>>,
    str_literals: &'hir Vec<String>,
    vtables: &'hir mir::VTables,
    /// Toplevel `self`
    the_main: Option<inkwell::values::BasicValueEnum<'ictx>>,
}

/// Compile hir and dump it to `outpath`
pub fn run(mir: &Mir, outpath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();
    let mut code_gen = CodeGen::new(&mir, &context, &module, &builder);
    code_gen.gen_program(&mir.hir)?;
    code_gen.module.print_to_file(outpath)?;
    Ok(())
}

impl<'hir: 'ictx, 'run, 'ictx: 'run> CodeGen<'hir, 'run, 'ictx> {
    pub fn new(
        mir: &'hir Mir,
        context: &'ictx inkwell::context::Context,
        module: &'run inkwell::module::Module<'ictx>,
        builder: &'run inkwell::builder::Builder<'ictx>,
    ) -> CodeGen<'hir, 'run, 'ictx> {
        CodeGen {
            context,
            module,
            builder,
            i1_type: context.bool_type(),
            i8_type: context.i8_type(),
            i8ptr_type: context.i8_type().ptr_type(AddressSpace::Generic),
            i32_type: context.i32_type(),
            i64_type: context.i64_type(),
            f64_type: context.f64_type(),
            void_type: context.void_type(),
            llvm_struct_types: HashMap::new(),
            str_literals: &mir.hir.str_literals,
            vtables: &mir.vtables,
            the_main: None,
        }
    }

    pub fn gen_program(&mut self, hir: &'hir Hir) -> Result<(), Error> {
        self.gen_declares();
        self.gen_class_structs(&hir.sk_classes);
        self.gen_string_literals(&hir.str_literals);
        self.gen_constant_ptrs(&hir.constants);
        self.gen_method_funcs(&hir.sk_methods);
        self.gen_vtables();
        self.gen_methods(&hir.sk_methods)?;
        self.gen_const_inits(&hir.const_inits)?;
        self.gen_user_main(&hir.main_exprs, &hir.main_lvars)?;
        self.gen_lambda_funcs(&hir)?;
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
        let fn_type = self.void_type.fn_type(&[self.i32_type.into()], false);
        self.module.add_function("exit", fn_type, None);

        let fn_type = self.void_type.fn_type(&[], false);
        self.module.add_function("GC_init", fn_type, None);
        let fn_type = self.i8ptr_type.fn_type(&[self.i64_type.into()], false);
        self.module.add_function("GC_malloc", fn_type, None);
        let fn_type = self
            .i8ptr_type
            .fn_type(&[self.i8ptr_type.into(), self.i64_type.into()], false);
        self.module.add_function("GC_realloc", fn_type, None);
        let fn_type = self.void_type.fn_type(
            &[
                self.i8ptr_type.into(),
                self.i8ptr_type.into(),
                self.i64_type.into(),
                self.i32_type.into(),
                self.i1_type.into(),
            ],
            false,
        );
        self.module
            .add_function("llvm.memcpy.p0i8.p0i8.i64", fn_type, None);

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
        global.set_initializer(&self.i8_type.const_array(&[
            self.i8_type.const_int(37, false),  // %
            self.i8_type.const_int(100, false), // d
            self.i8_type.const_int(0, false),
        ]));
        global.set_constant(true);

        let str_type = self.i8_type.array_type(3);
        let global = self.module.add_global(str_type, None, "putf_tmpl");
        global.set_linkage(inkwell::module::Linkage::Internal);
        global.set_initializer(&self.i8_type.const_array(&[
            self.i8_type.const_int(37, false),  // %
            self.i8_type.const_int(102, false), // f
            self.i8_type.const_int(0, false),
        ]));
        global.set_constant(true);
    }

    // Generate vtable constants
    fn gen_vtables(&self) {
        for (class_fullname, vtable) in self.vtables.iter() {
            let method_names = vtable.to_vec();
            let ary_type = self.i8ptr_type.array_type(method_names.len() as u32);
            let global = self
                .module
                .add_global(ary_type, None, &llvm_vtable_name(class_fullname));
            global.set_constant(true);
            global.set_linkage(inkwell::module::Linkage::Internal);
            let func_ptrs = method_names
                .iter()
                .map(|name| {
                    let func = self
                        .get_llvm_func(&name.full_name)
                        .as_any_value_enum()
                        .into_pointer_value();
                    self.builder
                        .build_bitcast(func, self.i8ptr_type, "")
                        .into_pointer_value()
                })
                .collect::<Vec<_>>();
            global.set_initializer(&self.i8ptr_type.const_array(&func_ptrs));
        }
    }

    fn gen_user_main(&mut self, main_exprs: &'hir HirExpressions, main_lvars: &'hir HirLVars) -> Result<(), Error> {
        // define void @user_main()
        let user_main_type = self.void_type.fn_type(&[], false);
        let function = self.module.add_function("user_main", user_main_type, None);
        // alloca
        let lvar_ptrs = self.gen_alloca_lvars(function, main_lvars);

        // CreateMain:
        let create_main_block = self.context.append_basic_block(function, "CreateMain");
        self.builder.build_unconditional_branch(create_main_block);
        self.builder.position_at_end(create_main_block);
        self.the_main = Some(self.allocate_sk_obj(&class_fullname("Object"), "main"));

        // UserMain:
        let user_main_block = self.context.append_basic_block(function, "UserMain");
        self.builder.build_unconditional_branch(user_main_block);
        self.builder.position_at_end(user_main_block);
        let mut ctx = CodeGenContext::new(function, FunctionOrigin::Other, None, lvar_ptrs);
        self.gen_exprs(&mut ctx, &main_exprs)?;
        self.builder.build_return(None);

        Ok(())
    }

    fn gen_main(&mut self) -> Result<(), Error> {
        // define i32 @main() {
        let main_type = self.i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", main_type, None);
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        // Call GC_init
        let func = self.get_llvm_func("GC_init");
        self.builder.build_call(func, &[], "");

        // Call init_constants, user_main
        let func = self.get_llvm_func("init_constants");
        self.builder.build_call(func, &[], "");
        let func = self.get_llvm_func("user_main");
        self.builder.build_call(func, &[], "");

        // ret i32 0
        self.builder
            .build_return(Some(&self.i32_type.const_int(0, false)));
        Ok(())
    }

    /// Create llvm struct types for Shiika objects
    fn gen_class_structs(&mut self, classes: &HashMap<ClassFullname, SkClass>) {
        // 1. Create struct type for each class
        for name in classes.keys() {
            self.llvm_struct_types
                .insert(name.clone(), self.context.opaque_struct_type(&name.0));
        }

        // 2. Set ivars
        let vt = self.llvm_vtable_ref_type().into();
        for (name, sk_class) in classes {
            let struct_type = self.llvm_struct_types.get(&name).unwrap();
            if name.0 == "Int" {
                struct_type.set_body(&[vt, self.i32_type.into()], false);
            } else if name.0 == "Float" {
                struct_type.set_body(&[vt, self.f64_type.into()], false);
            } else if name.0 == "Bool" {
                struct_type.set_body(&[vt, self.i1_type.into()], false);
            } else if name.0 == "Shiika::Internal::Ptr" {
                struct_type.set_body(&[vt, self.i8ptr_type.into()], false);
            } else {
                struct_type.set_body(&self.llvm_field_types(&sk_class.ivars), false);
            }
        }
    }

    /// List of fields of a class struct
    fn llvm_field_types(
        &self,
        ivars: &HashMap<String, SkIVar>,
    ) -> Vec<inkwell::types::BasicTypeEnum> {
        let mut values = ivars.values().collect::<Vec<_>>();
        values.sort_by_key(|ivar| ivar.idx);
        let mut types = values
            .iter()
            .map(|ivar| self.llvm_type(&ivar.ty))
            .collect::<Vec<_>>();
        types.insert(0, self.llvm_vtable_ref_type().into());
        types
    }

    /// Generate llvm constants for string literals
    fn gen_string_literals(&self, str_literals: &[String]) {
        str_literals.iter().enumerate().for_each(|(i, s)| {
            // PERF: how to avoid .to_string?
            let s_with_null = s.to_string() + "\0";
            let bytesize = s_with_null.len();
            let str_type = self.i8_type.array_type(bytesize as u32);
            let global = self
                .module
                .add_global(str_type, None, &format!("str_{}", i));
            global.set_linkage(inkwell::module::Linkage::Internal);
            let content = s_with_null
                .into_bytes()
                .iter()
                .map(|byte| self.i8_type.const_int((*byte).into(), false))
                .collect::<Vec<_>>();
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
                None => global.set_initializer(&null),
            }
        }
    }

    fn gen_const_inits(&self, const_inits: &'hir [HirExpression]) -> Result<(), Error> {
        // define void @"init_::XX"
        for expr in const_inits {
            match &expr.node {
                HirExpressionBase::HirConstAssign { fullname, .. } => {
                    let fn_type = self.void_type.fn_type(&[], false);
                    let function =
                        self.module
                            .add_function(&format!("init_{}", fullname.0), fn_type, None);
                    let mut ctx = CodeGenContext::new(function, FunctionOrigin::Other, None, HashMap::new());
                    let basic_block = self.context.append_basic_block(function, "");
                    self.builder.position_at_end(basic_block);
                    self.gen_expr(&mut ctx, &expr)?;
                    self.builder.build_return(None);
                }
                _ => panic!("gen_const_inits: Not a HirConstAssign"),
            }
        }

        // define void @init_constants()
        let fn_type = self.void_type.fn_type(&[], false);
        let function = self.module.add_function("init_constants", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        // call void @"init_::XX"()
        for expr in const_inits {
            match &expr.node {
                HirExpressionBase::HirConstAssign { fullname, .. } => {
                    let func = self.get_llvm_func(&format!("init_{}", fullname.0));
                    self.builder.build_call(func, &[], "");
                }
                _ => panic!("gen_const_inits: Not a HirConstAssign"),
            }
        }

        // Generate ::Void
        let ptr = self
            .module
            .get_global(&"::Void")
            .unwrap()
            .as_pointer_value();
        let value = self.allocate_sk_obj(&class_fullname("Void"), "void_obj");
        self.builder.build_store(ptr, value);

        self.builder.build_return(None);
        Ok(())
    }

    /// Create inkwell functions
    fn gen_method_funcs(&self, methods: &HashMap<ClassFullname, Vec<SkMethod>>) {
        methods.iter().for_each(|(cname, sk_methods)| {
            sk_methods.iter().for_each(|method| {
                let self_ty = cname.to_ty();
                let func_type = self.method_llvm_func_type(&self_ty, &method.signature);
                self.module
                    .add_function(&method.signature.fullname.full_name, func_type, None);
            })
        })
    }

    /// Return llvm funcion type of a method
    fn method_llvm_func_type(
        &self,
        self_ty: &TermTy,
        signature: &MethodSignature,
    ) -> inkwell::types::FunctionType<'ictx> {
        let param_tys = signature.params.iter().map(|p| &p.ty).collect::<Vec<_>>();
        self.llvm_func_type(Some(self_ty), &param_tys, &signature.ret_ty)
    }

    /// Return llvm funcion type
    fn llvm_func_type(
        &self,
        self_ty: Option<&TermTy>,
        param_tys: &[&TermTy],
        ret_ty: &TermTy,
    ) -> inkwell::types::FunctionType<'ictx> {
        let mut arg_types = param_tys
            .iter()
            .map(|ty| self.llvm_type(ty))
            .collect::<Vec<_>>();
        // Methods takes the self as the first argument
        if let Some(ty) = self_ty {
            arg_types.insert(0, self.llvm_type(ty));
        }

        if ret_ty.is_void_type() {
            self.void_type.fn_type(&arg_types, false)
        } else {
            let result_type = self.llvm_type(&ret_ty);
            result_type.fn_type(&arg_types, false)
        }
    }

    fn gen_methods(
        &self,
        methods: &'hir HashMap<ClassFullname, Vec<SkMethod>>,
    ) -> Result<(), Error> {
        methods.values().try_for_each(|sk_methods| {
            sk_methods
                .iter()
                .try_for_each(|method| self.gen_method(&method))
        })
    }

    fn gen_method(&self, method: &'hir SkMethod) -> Result<(), Error> {
        let func_name = &method.signature.fullname.full_name;
        self.gen_llvm_func_body(
            &func_name,
            &method.signature.params,
            Left(&method.body),
            &method.lvars,
            &method.signature.ret_ty,
        )
    }

    /// Generate body of a llvm function
    /// Used for methods and lambdas
    fn gen_llvm_func_body(
        &self,
        func_name: &str,
        params: &'hir [MethodParam],
        body: Either<&'hir SkMethodBody, &'hir HirExpressions>,
        lvars: &[(String, TermTy)],
        ret_ty: &TermTy,
    ) -> Result<(), Error> {
        // LLVM function
        let function = self.get_llvm_func(func_name);

        // Set param names
        for (i, param) in function.get_param_iter().enumerate() {
            if i == 0 {
                inkwell_set_name(param, "self")
            } else {
                inkwell_set_name(param, &params[i - 1].name)
            }
        }

        // alloca
        let lvar_ptrs = self.gen_alloca_lvars(function, lvars);

        // Method body
        match body {
            Left(method_body) => match method_body {
                SkMethodBody::RustMethodBody { gen } => gen(self, &function)?,
                SkMethodBody::RustClosureMethodBody { boxed_gen } => boxed_gen(self, &function)?,
                SkMethodBody::ShiikaMethodBody { exprs } => {
                    self.gen_shiika_method_body(function, None, ret_ty.is_void_type(), &exprs, lvar_ptrs)?
                }
            },
            Right(exprs) => {
                self.gen_shiika_lambda_body(function, Some(params), ret_ty.is_void_type(), &exprs, lvar_ptrs)?;
            }
        }
        Ok(())
    }

    /// Generate `alloca` section
    fn gen_alloca_lvars(
        &self,
        function: inkwell::values::FunctionValue,
        lvars: &[(String, TermTy)],
    ) -> HashMap<String, inkwell::values::PointerValue<'run>> {
        if lvars.is_empty() {
            let block = self.context.append_basic_block(function, "");
            self.builder.position_at_end(block);
            return HashMap::new()
        }
        let mut lvar_ptrs = HashMap::new();
        let alloca_start = self.context.append_basic_block(function, "alloca");
        self.builder.position_at_end(alloca_start);
        for (name, ty) in lvars {
            let ptr = self.builder.build_alloca(self.llvm_type(&ty), name);
            lvar_ptrs.insert(name.to_string(), ptr);
        }
        let alloca_end = self.context.append_basic_block(function, "alloca_End");
        self.builder.build_unconditional_branch(alloca_end);
        self.builder.position_at_end(alloca_end);
        lvar_ptrs
    }

    /// Generate body of llvm function of Shiika method
    fn gen_shiika_method_body(
        &self,
        function: inkwell::values::FunctionValue<'run>,
        function_params: Option<&'hir [MethodParam]>,
        void_method: bool,
        exprs: &'hir HirExpressions,
        lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    ) -> Result<(), Error> {
        let mut ctx = CodeGenContext::new(function, FunctionOrigin::Method, function_params, lvars);
        let last_value = self.gen_exprs(&mut ctx, exprs)?;
        if void_method {
            self.builder.build_return(None);
        } else {
            self.builder.build_return(Some(&last_value));
        }
        Ok(())
    }

    /// Generate body of llvm function of Shiika lambda
    fn gen_shiika_lambda_body(
        &self,
        function: inkwell::values::FunctionValue<'run>,
        function_params: Option<&'hir [MethodParam]>,
        void_method: bool,
        exprs: &'hir HirExpressions,
        lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    ) -> Result<(), Error> {
        let mut ctx = CodeGenContext::new(function, FunctionOrigin::Lambda, function_params, lvars);
        let last_value = self.gen_exprs(&mut ctx, exprs)?;
        if void_method {
            self.builder.build_return(None);
        } else {
            let llvm_type = self.llvm_type(&exprs.ty);
            let v = self.builder.build_bitcast(last_value, llvm_type, "");
            self.builder.build_return(Some(&v));
        }
        Ok(())
    }

    /// LLVM type of a reference to a vtable
    fn llvm_vtable_ref_type(&self) -> inkwell::types::PointerType {
        self.i8ptr_type
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
