mod boxing;
mod code_gen_context;
mod gen_exprs;
mod lambda;
mod utils;
pub mod values;
mod vtable;
mod wtable;
use crate::code_gen_context::*;
use crate::utils::*;
use crate::values::*;
use anyhow::{anyhow, Result};
use either::*;
use inkwell::types::*;
use inkwell::values::*;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::visitor::HirVisitor;
use skc_hir::*;
use skc_mir::{LibraryExports, Mir, VTables};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// CodeGen
///
/// 'hir > 'ictx > 'run
///
/// 'hir: the Hir
/// 'ictx: inkwell context
/// 'run: code_gen::run() after 'ictx is created
///
/// Basically inkwell types has 'ictx and inkwell values has 'run.
pub struct CodeGen<'hir: 'ictx, 'run, 'ictx: 'run> {
    pub generate_main: bool,
    pub context: &'ictx inkwell::context::Context,
    pub module: &'run inkwell::module::Module<'ictx>,
    pub builder: &'run inkwell::builder::Builder<'ictx>,
    pub i1_type: inkwell::types::IntType<'ictx>,
    pub i8_type: inkwell::types::IntType<'ictx>,
    pub ptr_type: inkwell::types::PointerType<'ictx>,
    pub i32_type: inkwell::types::IntType<'ictx>,
    pub i64_type: inkwell::types::IntType<'ictx>,
    pub f64_type: inkwell::types::FloatType<'ictx>,
    pub void_type: inkwell::types::VoidType<'ictx>,
    pub llvm_struct_types: HashMap<TypeFullname, inkwell::types::StructType<'ictx>>,
    str_literals: &'hir Vec<String>,
    vtables: &'hir VTables,
    imported_vtables: &'hir VTables,
    /// Toplevel `self`
    the_main: Option<SkObj<'run>>,
}

/// Compile hir and dump it to `outpath`
pub fn run<P: AsRef<Path>>(
    mir: &Mir,
    bc_path: P,
    // Generate .ll if given
    opt_ll_path: Option<P>,
    generate_main: bool,
    opt_target_triple: Option<&inkwell::targets::TargetTriple>,
) -> Result<()> {
    let context = inkwell::context::Context::create();
    let module = context.create_module("main");
    if let Some(triple) = opt_target_triple {
        module.set_triple(triple);
    }
    let builder = context.create_builder();
    let mut code_gen = CodeGen::new(mir, &context, &module, &builder, &generate_main);
    code_gen.gen_program(&mir.hir, &mir.imports)?;
    code_gen.module.write_bitcode_to_path(bc_path.as_ref());
    if let Some(ll_path) = opt_ll_path {
        code_gen
            .module
            .print_to_file(ll_path)
            .map_err(|llvm_str| anyhow!("{}", llvm_str.to_string()))?;
    }
    Ok(())
}

impl<'hir: 'ictx, 'run, 'ictx: 'run> CodeGen<'hir, 'run, 'ictx> {
    pub fn new(
        mir: &'hir Mir,
        context: &'ictx inkwell::context::Context,
        module: &'run inkwell::module::Module<'ictx>,
        builder: &'run inkwell::builder::Builder<'ictx>,
        generate_main: &bool,
    ) -> CodeGen<'hir, 'run, 'ictx> {
        CodeGen {
            generate_main: *generate_main,
            context,
            module,
            builder,
            i1_type: context.bool_type(),
            i8_type: context.i8_type(),
            ptr_type: context.i8_type().ptr_type(Default::default()),
            i32_type: context.i32_type(),
            i64_type: context.i64_type(),
            f64_type: context.f64_type(),
            void_type: context.void_type(),
            llvm_struct_types: HashMap::new(),
            str_literals: &mir.hir.str_literals,
            vtables: &mir.vtables,
            imported_vtables: &mir.imports.vtables,
            the_main: None,
        }
    }

    pub fn gen_program(&mut self, hir: &'hir Hir, imports: &LibraryExports) -> Result<()> {
        self.gen_declares();
        self.define_class_class();
        self.gen_imports(imports);
        self.gen_type_structs(&hir.sk_types);
        self.gen_lambda_capture_structs(hir)?;
        self.gen_string_literals(&hir.str_literals);
        self.gen_constant_ptrs(&hir.constants);
        self.gen_boxing_funcs();
        self.gen_method_funcs(&hir.sk_methods);
        self.gen_vtables();
        self.gen_wtables(&hir.sk_types);
        self.gen_insert_wtables(&hir.sk_types);
        self.gen_methods(&hir.sk_methods)?;
        self.gen_const_inits(&hir.const_inits)?;
        if self.generate_main {
            self.gen_init_constants(&hir.const_inits, true);
            self.gen_user_main(&hir.main_exprs, &hir.main_lvars)?;
            self.gen_main();
        } else {
            // generating builtin
            self.gen_init_constants(&hir.const_inits, false);
            self.impl_boxing_funcs();
        }
        self.gen_lambda_funcs(hir)?;
        Ok(())
    }

    fn gen_declares(&self) {
        let fn_type = self.void_type.fn_type(&[], false);
        self.module.add_function("GC_init", fn_type, None);
        let fn_type = self.ptr_type.fn_type(&[self.i64_type.into()], false);
        self.module.add_function("shiika_malloc", fn_type, None);
        let fn_type = self
            .ptr_type
            .fn_type(&[self.ptr_type.into(), self.i64_type.into()], false);
        self.module.add_function("shiika_realloc", fn_type, None);

        let fn_type = self.ptr_type.fn_type(
            &[
                self.ptr_type.into(),
                self.i64_type.into(),
                self.i64_type.into(),
            ],
            false,
        );
        self.module
            .add_function("shiika_lookup_wtable", fn_type, None);

        let fn_type = self.ptr_type.fn_type(
            &[
                self.ptr_type.into(),
                self.i64_type.into(),
                self.ptr_type.into(),
                self.i64_type.into(),
            ],
            false,
        );
        self.module
            .add_function("shiika_insert_wtable", fn_type, None);
    }

    /// Define llvm struct type for `Class` in advance
    fn define_class_class(&mut self) {
        self.llvm_struct_types.insert(
            type_fullname("Class"),
            self.context.opaque_struct_type("Class"),
        );
    }

    /// Generate information to use imported items
    fn gen_imports(&mut self, imports: &LibraryExports) {
        self.gen_import_classes(&imports.sk_types);
        self.gen_import_vtables(&imports.vtables);
        self.gen_import_constants(&imports.constants);
    }

    /// Generate LLVM types and `declare`s for imported class/modules
    fn gen_import_classes(&mut self, imported_types: &SkTypes) {
        // LLVM type
        for name in imported_types.types.keys() {
            self.llvm_struct_types
                .insert(name.clone(), self.context.opaque_struct_type(&name.0));
        }
        self.define_type_struct_fields(imported_types);

        // Methods
        for (typename, sk_type) in &imported_types.types {
            for (sig, _) in sk_type.base().method_sigs.unordered_iter() {
                let func_type = self.method_llvm_func_type(&sk_type.erasure().to_term_ty(), sig);
                let func_name = typename.method_fullname(&sig.fullname.first_name);
                self.module
                    .add_function(&method_func_name(&func_name).0, func_type, None);
            }
        }
    }

    /// Declare `external global` for vtable of each class
    fn gen_import_vtables(&self, vtables: &VTables) {
        for (fullname, vtable) in vtables.iter() {
            let name = llvm_vtable_const_name(fullname);
            let ary_type = self.ptr_type.array_type(vtable.size() as u32);
            let _global = self.module.add_global(ary_type, None, &name);
        }
    }

    /// Declare `external global` for each imported constant
    fn gen_import_constants(&self, imported_constants: &HashMap<ConstFullname, TermTy>) {
        for (fullname, _) in imported_constants {
            let name = llvm_const_name(fullname);
            let global = self.module.add_global(self.llvm_type(), None, &name);
            global.set_linkage(inkwell::module::Linkage::External);
            // @init_::XX
            let fn_type = self.void_type.fn_type(&[], false);
            self.module
                .add_function(&const_initialize_func_name(fullname), fn_type, None);
        }
    }

    /// Generate vtable constants
    fn gen_vtables(&self) {
        for (class_fullname, vtable) in self.vtables.iter() {
            let method_names = vtable.to_vec();
            let ary_type = self.ptr_type.array_type(method_names.len() as u32);
            let tmp = llvm_vtable_const_name(class_fullname);
            let global = self.module.add_global(ary_type, None, &tmp);
            global.set_constant(true);
            let func_ptrs = method_names
                .iter()
                .map(|name| {
                    let func = self
                        .get_llvm_func(&method_func_name(name))
                        .as_global_value()
                        .as_pointer_value();
                    self.builder
                        .build_bitcast(func, self.ptr_type, "")
                        .into_pointer_value()
                })
                .collect::<Vec<_>>();
            global.set_initializer(&self.ptr_type.const_array(&func_ptrs));
        }
    }

    /// Generate wtable constants
    fn gen_wtables(&self, sk_types: &SkTypes) {
        for sk_class in sk_types.sk_classes() {
            wtable::gen_wtable_constants(self, sk_class);
        }
    }

    /// Generate functions to insert wtables
    fn gen_insert_wtables(&self, sk_types: &SkTypes) {
        for sk_class in sk_types.sk_classes() {
            if !sk_class.wtable.is_empty() {
                wtable::gen_insert_wtable(self, sk_class);
            }
        }
    }

    /// Generate `init_constants()`
    // TODO: imported_constants should be Vec (order matters)
    fn gen_init_constants(&self, const_inits: &'hir [HirExpression], is_main: bool) {
        let package_name = if is_main { "main" } else { "builtin" };
        // define void @xxx_init_constants()
        let fn_type = self.void_type.fn_type(&[], false);
        let function =
            self.module
                .add_function(&format!("{}_init_constants", package_name), fn_type, None);
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        // Initialize imported constants
        if is_main {
            let imports = vec!["builtin"];
            for s in imports {
                let fn_type = self.void_type.fn_type(&[], false);
                self.module
                    .add_function(&format!("{}_init_constants", s), fn_type, None);
                let func = self.get_llvm_func(&llvm_func_name(format!("{}_init_constants", s)));
                self.builder.build_direct_call(func, &[], "");
            }
        }

        // Initialize own constants
        let basic_classes = vec!["Metaclass", "Class", "Shiika::Internal::Ptr"]
            .into_iter()
            .map(const_fullname)
            .collect::<Vec<_>>();
        if !is_main {
            // These builtin classes must be created first
            for name in &basic_classes {
                let func = self.get_llvm_func(&llvm_func_name(const_initialize_func_name(name)));
                self.builder.build_direct_call(func, &[], "");
            }
        }
        for expr in const_inits {
            match &expr.node {
                HirExpressionBase::HirConstAssign { fullname, .. } => {
                    if !basic_classes.iter().any(|s| s.0 == fullname.0) {
                        let func = self
                            .get_llvm_func(&llvm_func_name(const_initialize_func_name(fullname)));
                        self.builder.build_direct_call(func, &[], "");
                    }
                }
                _ => panic!("gen_init_constants: Not a HirConstAssign"),
            }
        }

        self.builder.build_return(None);
    }

    #[allow(clippy::ptr_arg)]
    fn gen_user_main(
        &mut self,
        main_exprs: &'hir Vec<HirExpression>,
        main_lvars: &'hir HirLVars,
    ) -> Result<()> {
        // define void @user_main()
        let user_main_type = self.void_type.fn_type(&[], false);
        let function = self.module.add_function("user_main", user_main_type, None);
        let block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(block);

        // alloca
        let mut lvars = main_lvars.clone();
        for expr in main_exprs {
            lvars.append(&mut CollectLVarsVisitor::run(expr)?);
        }
        let lvar_ptrs = self.gen_alloca_lvars(function, &lvars);

        // CreateMain:
        let create_main_block = self.context.append_basic_block(function, "CreateMain");
        self.builder.build_unconditional_branch(create_main_block);
        self.builder.position_at_end(create_main_block);
        self.the_main = Some(self.allocate_sk_obj(&class_fullname("Object")));

        // UserMain:
        let user_main_block = self.context.append_basic_block(function, "UserMain");
        self.builder.build_unconditional_branch(user_main_block);
        self.builder.position_at_end(user_main_block);

        let (end_block, mut ctx) = self.new_ctx(FunctionOrigin::Other, function, lvar_ptrs);
        for expr in main_exprs {
            let value = self.gen_expr(&mut ctx, expr)?;
            if value.is_none() {
                // Found `return`, `panic` or something. The rest of expressions
                // will never be executed
                break;
            }
        }
        self.builder.build_unconditional_branch(*end_block);
        self.builder.position_at_end(*end_block);
        self.builder.build_return(None);

        Ok(())
    }

    fn gen_main(&mut self) {
        // define i32 @main() {
        let main_type = self.i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", main_type, None);
        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        // Call GC_init
        let func = self.get_llvm_func(&llvm_func_name("GC_init"));
        self.builder.build_direct_call(func, &[], "");

        // Call init_constants, user_main
        let func = self.get_llvm_func(&llvm_func_name("main_init_constants"));
        self.builder.build_direct_call(func, &[], "");
        let func = self.get_llvm_func(&llvm_func_name("user_main"));
        self.builder.build_direct_call(func, &[], "");

        // ret i32 0
        self.builder
            .build_return(Some(&self.i32_type.const_int(0, false)));
    }

    /// Create llvm struct types for Shiika objects
    fn gen_type_structs(&mut self, sk_types: &SkTypes) {
        // Create all the struct types in advance (because it may be used as other class's ivar)
        for name in sk_types.types.keys() {
            self.llvm_struct_types
                .insert(name.clone(), self.context.opaque_struct_type(&name.0));
        }

        self.define_type_struct_fields(sk_types);
    }

    /// Set fields for ivars
    fn define_type_struct_fields(&self, sk_types: &SkTypes) {
        let vt = self.llvm_vtable_ref_type().into();
        let ct = self.class_object_ref_type().into();
        for (name, sk_type) in &sk_types.types {
            let struct_type = self.llvm_struct_types.get(name).unwrap();
            match sk_type {
                SkType::Class(class) => match name.0.as_str() {
                    "Int" => {
                        struct_type.set_body(&[vt, ct, self.i64_type.into()], false);
                    }
                    "Float" => {
                        struct_type.set_body(&[vt, ct, self.f64_type.into()], false);
                    }
                    "Bool" => {
                        struct_type.set_body(&[vt, ct, self.i1_type.into()], false);
                    }
                    "Shiika::Internal::Ptr" => {
                        struct_type.set_body(&[vt, ct, self.ptr_type.into()], false);
                    }
                    _ => {
                        struct_type.set_body(&self.llvm_field_types(&class.ivars), false);
                    }
                },
                SkType::Module(_) => {
                    // For modules, insert only basic fields
                    struct_type.set_body(&self.llvm_field_types(&Default::default()), false);
                }
            }
        }
    }

    /// List of fields of a class struct
    fn llvm_field_types(
        &self,
        ivars: &HashMap<String, SkIVar>,
    ) -> Vec<inkwell::types::BasicTypeEnum<'_>> {
        let mut values = ivars.values().collect::<Vec<_>>();
        values.sort_by_key(|ivar| ivar.idx);
        let mut types = vec![self.llvm_type(); values.len()];
        types.insert(0, self.llvm_vtable_ref_type().into());
        types.insert(1, self.class_object_ref_type().into());
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

    /// Generate llvm global that holds Shiika constants
    fn gen_constant_ptrs(&self, constants: &HashMap<ConstFullname, TermTy>) {
        for (fullname, _) in constants {
            let name = llvm_const_name(fullname);
            let global = self.module.add_global(self.llvm_type(), None, &name);
            let null = self.llvm_type().into_pointer_type().const_null();
            global.set_initializer(&null);
        }
    }

    /// Define `void @"init_::XX"`
    fn gen_const_inits(&self, const_inits: &'hir [HirExpression]) -> Result<()> {
        for expr in const_inits {
            match &expr.node {
                HirExpressionBase::HirConstAssign { fullname, .. } => {
                    let fn_type = self.void_type.fn_type(&[], false);
                    let function = self.module.add_function(
                        &const_initialize_func_name(fullname),
                        fn_type,
                        None,
                    );
                    let basic_block = self.context.append_basic_block(function, "");
                    self.builder.position_at_end(basic_block);
                    let (end_block, mut ctx) =
                        self.new_ctx(FunctionOrigin::Other, function, HashMap::new());
                    self.gen_expr(&mut ctx, expr)?;
                    self.builder.build_unconditional_branch(*end_block);
                    self.builder.position_at_end(*end_block);
                    self.builder.build_return(None);
                }
                _ => panic!("gen_const_inits: Not a HirConstAssign"),
            }
        }

        Ok(())
    }

    /// Create inkwell functions
    fn gen_method_funcs(&self, methods: &HashMap<TypeFullname, Vec<SkMethod>>) {
        methods.iter().for_each(|(tname, sk_methods)| {
            sk_methods.iter().for_each(|method| {
                let self_ty = tname.to_ty();
                let func_type = self.method_llvm_func_type(&self_ty, &method.signature);
                let func_name = method_func_name(&method.signature.fullname);
                self.module.add_function(&func_name.0, func_type, None);
            })
        })
    }

    /// Return llvm funcion type of a method
    fn method_llvm_func_type(
        &self,
        self_ty: &TermTy,
        signature: &MethodSignature,
    ) -> inkwell::types::FunctionType<'ictx> {
        let mut param_tys = signature
            .params
            .iter()
            .map(|p| p.ty.clone())
            .collect::<Vec<_>>();
        if signature.is_the_new() {
            // HACK: Generic .new (eg. Pair.new) has type parameters but they will never be called because calls of
            // them are replaced by corresponding specialized one (eg. Pair<Int,Int>.new). Since
            // specialized .new's does not need type arguments, skip adding extranous arguments for
            // tyargs.
        } else {
            for tp in ty::typarams_to_typaram_refs(&signature.typarams, TyParamKind::Method, true) {
                param_tys.push(tp.to_term_ty());
            }
        }

        self.llvm_func_type(Some(self_ty), &param_tys, &signature.ret_ty)
    }

    /// Return llvm funcion type
    fn llvm_func_type<T: AsRef<TermTy>>(
        &self,
        self_ty: Option<&TermTy>,
        param_tys: &[T],
        ret_ty: &TermTy,
    ) -> inkwell::types::FunctionType<'ictx> {
        let mut arg_types = vec![self.llvm_type().into(); param_tys.len()];
        // Methods takes the self as the first argument
        if self_ty.is_some() {
            arg_types.insert(0, self.llvm_type().into());
        }

        if ret_ty.is_never_type() {
            // `Never` does not have an instance
            self.void_type.fn_type(&arg_types, false)
        } else {
            self.llvm_type().fn_type(&arg_types, false)
        }
    }

    fn gen_methods(&self, methods: &'hir HashMap<TypeFullname, Vec<SkMethod>>) -> Result<()> {
        methods.values().try_for_each(|sk_methods| {
            sk_methods
                .iter()
                .try_for_each(|method| self.gen_method(method))
        })
    }

    fn gen_method(&self, method: &'hir SkMethod) -> Result<()> {
        log::trace!("method {}", &method.signature);
        if method.is_rustlib() {
            return Ok(());
        }
        let func_name = method_func_name(&method.signature.fullname);
        self.gen_llvm_func_body(
            &func_name,
            &method.signature.params,
            &method.signature.typarams,
            Left(&method.body),
            &method.lvars,
            &method.signature.ret_ty,
            None,
        )
    }

    /// Generate body of a llvm function
    /// Used for methods and lambdas
    #[allow(clippy::too_many_arguments)]
    fn gen_llvm_func_body(
        &self,
        func_name: &LlvmFuncName,
        params: &'hir [MethodParam],
        typarams: &'hir [TyParam],
        body: Either<&'hir SkMethodBody, &'hir HirExpression>,
        lvars: &HirLVars,
        ret_ty: &TermTy,
        lambda_name: Option<String>,
    ) -> Result<()> {
        let is_lambda = lambda_name.is_some();
        // LLVM function
        // (Function for lambdas are created in gen_lambda_expr)
        let function = self.get_llvm_func(func_name);
        let block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(block);

        // Set param names
        for (i, param) in function.get_param_iter().enumerate() {
            let name = llvm_func_param_name(is_lambda, params, typarams, i);
            inkwell_set_name(param, name);
        }

        // alloca
        let mut lvars = lvars.clone();
        let mut more_lvars = match body {
            Left(SkMethodBody::Normal { exprs }) | Right(exprs) => CollectLVarsVisitor::run(exprs)?,
            _ => Default::default(),
        };
        lvars.append(&mut more_lvars);
        let lvar_ptrs = self.gen_alloca_lvars(function, &lvars);

        // Method body
        match body {
            Left(method_body) => match method_body {
                SkMethodBody::Normal { exprs } => self.gen_shiika_function_body(
                    function,
                    FunctionOrigin::Method { params },
                    ret_ty,
                    exprs,
                    lvar_ptrs,
                )?,
                SkMethodBody::RustLib => (),
                SkMethodBody::New {
                    classname,
                    initializer,
                    arity,
                    const_is_obj,
                } => self.gen_body_of_new(
                    function.get_params(),
                    classname,
                    initializer,
                    *arity,
                    *const_is_obj,
                ),
                SkMethodBody::Getter {
                    name,
                    self_ty,
                    idx,
                    ty,
                } => {
                    let this = self.get_nth_param(self_ty.clone(), &function, 0);
                    let val = self.build_ivar_load(this, ty.clone(), *idx, name);
                    self.build_return(&val);
                }
                SkMethodBody::Setter {
                    name,
                    ty,
                    self_ty,
                    idx,
                } => {
                    let this = self.get_nth_param(self_ty.clone(), &function, 0);
                    let val = self.get_nth_param(ty.clone(), &function, 1);
                    self.build_ivar_store(this, *idx, name, val.clone());
                    self.build_return(&val);
                }
            },
            Right(exprs) => {
                self.gen_shiika_function_body(
                    function,
                    FunctionOrigin::Lambda {
                        name: lambda_name.unwrap(),
                        params,
                    },
                    ret_ty,
                    exprs,
                    lvar_ptrs,
                )?;
            }
        }
        Ok(())
    }

    /// Generate `alloca` section
    fn gen_alloca_lvars(
        &self,
        function: inkwell::values::FunctionValue,
        lvars: &[HirLVar],
    ) -> HashMap<String, inkwell::values::PointerValue<'run>> {
        if lvars.is_empty() {
            return HashMap::new();
        }
        let mut lvar_ptrs = HashMap::new();
        let alloca_start = self.context.append_basic_block(function, "alloca");
        self.builder.build_unconditional_branch(alloca_start);
        self.builder.position_at_end(alloca_start);
        for HirLVar { name, captured, .. } in lvars {
            let obj_ty = self.llvm_type();
            if *captured {
                // Allocate memory on heap in case it lives longer than the method call.
                let ptrptr = self.allocate_llvm_obj(&obj_ty).into_pointer_value();
                lvar_ptrs.insert(name.to_string(), ptrptr);
            } else {
                let ptr = self.builder.build_alloca(obj_ty, name);
                lvar_ptrs.insert(name.to_string(), ptr);
            }
        }
        let alloca_end = self.context.append_basic_block(function, "alloca_End");
        self.builder.build_unconditional_branch(alloca_end);
        self.builder.position_at_end(alloca_end);
        lvar_ptrs
    }

    /// Generate body of llvm function of Shiika method or lambda
    fn gen_shiika_function_body(
        &self,
        function: inkwell::values::FunctionValue<'run>,
        function_origin: FunctionOrigin<'hir>,
        ret_ty: &TermTy,
        exprs: &'hir HirExpression,
        lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    ) -> Result<()> {
        let (end_block, mut ctx) = self.new_ctx(function_origin, function, lvars);
        let (last_value, last_value_block) = if let Some(v) = self.gen_expr(&mut ctx, exprs)? {
            let b = self.context.append_basic_block(ctx.function, "Ret");
            self.builder.build_unconditional_branch(b);
            self.builder.position_at_end(b);
            let last_value = self.bitcast(v, ret_ty, "as");
            self.builder.build_unconditional_branch(*end_block);
            (Some(last_value), Some(b))
        } else {
            (None, None)
        };

        self.builder.position_at_end(*end_block);

        if ret_ty.is_never_type() {
            // `Never` does not have an instance
            self.builder.build_return(None);
        } else if last_value.is_none() && ctx.returns.is_empty() {
            // `exprs` ends with `panic` and there is no `return`
            let null = self.llvm_type().into_pointer_type().const_null();
            self.builder.build_return(Some(&null));
        } else if ret_ty.is_void_type() {
            self.build_return_void();
        } else {
            // Make a phi node from the `return`s
            let mut incomings = ctx
                .returns
                .iter()
                .map(|(v, b)| (&v.0 as &dyn inkwell::values::BasicValue, *b))
                .collect::<Vec<_>>();
            let v;
            if let Some(b) = last_value_block {
                v = last_value.unwrap();
                incomings.push((&v.0, b));
            }
            let phi_node = self.builder.build_phi(self.llvm_type(), "methodResult");
            phi_node.add_incoming(incomings.as_slice());
            self.builder.build_return(Some(&phi_node.as_basic_value()));
        }
        Ok(())
    }

    /// LLVM type of a reference to a vtable
    fn llvm_vtable_ref_type(&self) -> inkwell::types::PointerType<'_> {
        self.ptr_type
    }

    /// LLVM type of a reference to a class object
    fn class_object_ref_type(&self) -> inkwell::types::PointerType<'_> {
        self.llvm_type().into_pointer_type()
    }

    /// Generate body of `.new`
    pub fn gen_body_of_new(
        &self,
        llvm_func_args: Vec<inkwell::values::BasicValueEnum>,
        class_fullname: &ClassFullname,
        initializer: &Option<MethodSignature>,
        arity: usize,
        _const_is_obj: bool,
    ) {
        // Allocate memory and set .class (which is the receiver of .new)
        let class_obj = SkClassObj(llvm_func_args[0].into_pointer_value());
        let obj = self._allocate_sk_obj(class_fullname, class_obj);

        // Call initialize
        if let Some(initialize_sig) = initializer {
            let init_cls_name = initialize_sig.fullname.type_name.to_class_fullname();
            let addr = if init_cls_name == *class_fullname {
                obj.clone()
            } else {
                // `initialize` is defined in an ancestor class. Bitcast is needed
                // to pass the obj to the `initialize` func
                let ances_type = self
                    .llvm_struct_types
                    .get(&init_cls_name.to_type_fullname())
                    .expect("ances_type not found")
                    .ptr_type(Default::default());
                SkObj::new(
                    init_cls_name.to_ty(),
                    self.builder
                        .build_bitcast(obj.clone().0, ances_type, "obj_as_super"),
                )
            };
            let args = (0..=arity)
                .map(|i| {
                    if i == 0 {
                        addr.0.into()
                    } else {
                        llvm_func_args[i].into()
                    }
                })
                .collect::<Vec<_>>();
            let initialize = self.get_llvm_func(&method_func_name(&initialize_sig.fullname));
            self.builder.build_direct_call(initialize, &args, "");
        };

        self.build_return(&obj);
    }

    /// Create a CodeGenContext
    fn new_ctx(
        &self,
        origin: FunctionOrigin<'hir>,
        function: inkwell::values::FunctionValue<'run>,
        lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    ) -> (
        Rc<inkwell::basic_block::BasicBlock<'run>>,
        CodeGenContext<'hir, 'run>,
    ) {
        let end_block = self.context.append_basic_block(function, "End");
        let ref_end_block1 = Rc::new(end_block);
        let ref_end_block2 = Rc::clone(&ref_end_block1);
        let ctx = CodeGenContext::new(function, ref_end_block1, origin, lvars);
        (ref_end_block2, ctx)
    }
}

fn llvm_func_param_name<'hir>(
    is_lambda: bool,
    params: &'hir [MethodParam],
    typarams: &'hir [TyParam],
    i: usize,
) -> &'hir str {
    if i == 0 {
        return if is_lambda { "fn_x" } else { "self" };
    }
    let n_params = params.len();
    if i <= n_params {
        &params[i - 1].name
    } else {
        &typarams[i - (n_params + 1)].name
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

fn const_initialize_func_name(name: &ConstFullname) -> String {
    format!("init_{}", &name.0[2..])
}

struct CollectLVarsVisitor(HirLVars);
impl CollectLVarsVisitor {
    fn run(e: &HirExpression) -> Result<HirLVars> {
        let mut v = CollectLVarsVisitor(Default::default());
        visitor::walk_expr(&mut v, e)?;
        Ok(v.0)
    }
}
impl HirVisitor<'_> for CollectLVarsVisitor {
    fn visit_expr(&mut self, expr: &HirExpression) -> Result<()> {
        match &expr.node {
            HirExpressionBase::HirIfExpression { lvars, .. } => self.0.append(&mut lvars.clone()),
            HirExpressionBase::HirWhileExpression { lvars, .. } => {
                self.0.append(&mut lvars.clone())
            }
            _ => (),
        }
        Ok(())
    }
}
