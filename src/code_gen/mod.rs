mod code_gen_context;
use std::collections::HashMap;
use std::rc::Rc;
use inkwell::AddressSpace;
use inkwell::values::*;
use inkwell::types::*;
use crate::error;
use crate::error::Error;
use crate::ty;
use crate::ty::*;
use crate::hir::*;
use crate::hir::HirExpressionBase::*;
use crate::names::*;
use crate::code_gen::code_gen_context::*;

pub struct CodeGen {
    pub context: inkwell::context::Context,
    pub module: inkwell::module::Module,
    pub builder: inkwell::builder::Builder,
    pub i1_type: inkwell::types::IntType,
    pub i32_type: inkwell::types::IntType,
    pub i64_type: inkwell::types::IntType,
    pub f64_type: inkwell::types::FloatType,
    pub void_type: inkwell::types::VoidType,
    llvm_struct_types: HashMap<ClassFullname, inkwell::types::StructType>,
    /// Toplevel `self`
    the_main: Option<inkwell::values::BasicValueEnum>,
}

impl CodeGen {
    pub fn new() -> CodeGen {
        let context = inkwell::context::Context::create();
        let module = context.create_module("main");
        let builder = context.create_builder();
        CodeGen {
            context: context,
            module: module,
            builder: builder,
            i1_type: inkwell::types::IntType::bool_type(),
            i32_type: inkwell::types::IntType::i32_type(),
            i64_type: inkwell::types::IntType::i64_type(),
            f64_type: inkwell::types::FloatType::f64_type(),
            void_type: inkwell::types::VoidType::void_type(),
            llvm_struct_types: HashMap::new(),
            the_main: None,
        }
    }

    pub fn gen_program(&mut self, hir: Hir) -> Result<(), Error> {
        self.gen_declares();
        self.gen_class_structs(&hir.sk_classes);
        self.gen_method_funcs(&hir.sk_methods);
        self.gen_methods(&hir.sk_methods)?;
        self.gen_constant_ptrs(&hir.constants);
        self.gen_main(&hir.main_exprs)?;
        Ok(())
    }

    fn gen_declares(&self) {
        let fn_type = self.i32_type.fn_type(&[self.i32_type.into()], false);
        self.module.add_function("putchar", fn_type, None);

        let fn_type = self.void_type.fn_type(&[], false);
        self.module.add_function("GC_init", fn_type, None);

        let fn_type = IntType::i8_type().ptr_type(AddressSpace::Generic).fn_type(&[IntType::i64_type().into()], false);

        self.module.add_function("GC_malloc", fn_type, None);

        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("sin", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("cos", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("sqrt", fn_type, None);
        let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
        self.module.add_function("fabs", fn_type, None);
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

    fn gen_main(&mut self, main_exprs: &HirExpressions) -> Result<(), Error> {
        // define i32 @main() {
        let main_type = self.i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", main_type, None);
        let basic_block = self.context.append_basic_block(&function, "");
        self.builder.position_at_end(&basic_block);

        // Call GC_init
        let func = self.module.get_function("GC_init").unwrap();
        self.builder.build_call(func, &[], "");

        // Create the Main object
        self.the_main = Some(self.allocate_sk_obj(&ClassFullname("Object".to_string())));

        // Generate main exprs
        let mut ctx = CodeGenContext::new(function);
        self.gen_exprs(&mut ctx, &main_exprs)?;

        // ret i32 0
        self.builder.build_return(Some(&self.i32_type.const_int(0, false)));
        Ok(())
    }

    /// Create llvm struct types for Shiika objects
    fn gen_class_structs(&mut self, classes: &HashMap<ClassFullname, SkClass>) {
        classes.values().for_each(|sk_class| {
            self.llvm_struct_types.insert(
                sk_class.fullname.clone(),
                self.llvm_struct_type(&sk_class.fullname.0));
        })
    }

    /// Create inkwell functions
    fn gen_method_funcs(&self,
                        methods: &HashMap<ClassFullname, Vec<SkMethod>>) {
        methods.iter().for_each(|(cname, sk_methods)| {
            sk_methods.iter().for_each(|method| {
                let self_ty = ty::raw(&cname.0);
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
            .expect(&format!("[BUG] get_function not found: {:?}", method.signature));

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
        let last_value_opt = self.gen_exprs(&mut ctx, exprs)?;
        if void_method {
            self.builder.build_return(None);
        }
        else {
            match last_value_opt {
                Some(v) => self.builder.build_return(Some(&v)),
                None => self.builder.build_return(None)
            };
        }
        Ok(())
    }

    fn gen_exprs(&self,
                ctx: &mut CodeGenContext,
                exprs: &HirExpressions) -> Result<Option<inkwell::values::BasicValueEnum>, Error> {
        let mut last_value_opt = None;
        exprs.exprs.iter().try_for_each(|expr| {
            let value: inkwell::values::BasicValueEnum = self.gen_expr(ctx, &expr)?;

            last_value_opt = Some(value);
            Ok(())
        })?;
        Ok(last_value_opt)
    }

    fn gen_expr(&self,
                ctx: &mut CodeGenContext,
                expr: &HirExpression) -> Result<inkwell::values::BasicValueEnum, Error> {
        match &expr.node {
            HirIfExpression { cond_expr, then_expr, else_expr } => {
                self.gen_if_expr(ctx, &expr.ty, &cond_expr, &then_expr, &else_expr)
            },
            HirWhileExpression { cond_expr, body_exprs } => {
                self.gen_while_expr(ctx, &cond_expr, &body_exprs)
            },
            HirBreakExpression => {
                self.gen_break_expr(ctx)
            },
            HirLVarAssign { name, rhs } => {
                self.gen_lvar_assign(ctx, name, rhs)
            },
            HirConstAssign { fullname, rhs } => {
                self.gen_const_assign(ctx, fullname, rhs)
            },
            HirMethodCall { receiver_expr, method_fullname, arg_exprs } => {
                self.gen_method_call(ctx, method_fullname, receiver_expr, arg_exprs)
            },
            HirArgRef { idx } => {
                Ok(ctx.function.get_nth_param((*idx as u32) + 1).unwrap()) // +1 for the first %self 
            },
            HirLVarRef { name } => {
                self.gen_lvar_ref(ctx, name)
            },
            HirConstRef { fullname } => {
                let ptr = self.module.get_global(&fullname.0).
                    expect(&format!("[BUG] global for Constant `{}' not created", fullname.0)).
                    as_pointer_value();
                Ok(self.builder.build_load(ptr, &fullname.0))
            },
            HirSelfExpression => {
                if ctx.function.get_name().to_str().unwrap() == "main" {
                    Ok(self.the_main.unwrap())
                }
                else {
                    // The first arg of llvm function is `self`
                    Ok(ctx.function.get_first_param().unwrap())
                }
            },
            HirFloatLiteral { value } => {
                Ok(self.gen_float_literal(*value))
            },
            HirDecimalLiteral { value } => {
                Ok(self.gen_decimal_literal(*value))
            },
            HirBooleanLiteral { value } => {
                Ok(self.gen_boolean_literal(*value))
            },
            HirClassLiteral { fullname } => {
                Ok(self.gen_class_literal(fullname))
            }
            HirNop => {
                panic!("HirNop not handled by `else`")
            },
            _ => {
                panic!("TODO: {:?}", expr.node) 
            }
        }
    }

    fn gen_if_expr(&self, 
                   ctx: &mut CodeGenContext,
                   ty: &TermTy,
                   cond_expr: &HirExpression,
                   then_expr: &HirExpression,
                   else_expr: &HirExpression) -> Result<inkwell::values::BasicValueEnum, Error> {
        let cond_value = self.gen_expr(ctx, cond_expr)?.into_int_value();
        let then_block = ctx.function.append_basic_block(&"IfThen");
        let else_block = ctx.function.append_basic_block(&"IfElse");
        let merge_block = ctx.function.append_basic_block(&"IfEnd");
        self.builder.build_conditional_branch(cond_value, &then_block, &else_block);
        self.builder.position_at_end(&then_block);
        let then_value: &dyn inkwell::values::BasicValue = &self.gen_expr(ctx, then_expr)?;
        self.builder.build_unconditional_branch(&merge_block);
        let then_block = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(&else_block);
        let else_value = self.gen_expr(ctx, else_expr)?;
        self.builder.build_unconditional_branch(&merge_block);
        let else_block = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(&merge_block);

        if ty == &ty::raw("Never") {
            Ok(self.i1_type.const_int(0, false).as_basic_value_enum()) // dummy value
        }
        else {
            let phi_node = self.builder.build_phi(self.llvm_type(ty), "");
            phi_node.add_incoming(&[(then_value, &then_block), (&else_value, &else_block)]);
            Ok(phi_node.as_basic_value())
        }
    }

    fn gen_while_expr(&self, 
                      ctx: &mut CodeGenContext,
                      cond_expr: &HirExpression,
                      body_exprs: &HirExpressions) -> Result<inkwell::values::BasicValueEnum, Error> {

        let begin_block = ctx.function.append_basic_block(&"WhileBegin");
        self.builder.build_unconditional_branch(&begin_block);
        // WhileBegin:
        self.builder.position_at_end(&begin_block);
        let cond_value = self.gen_expr(ctx, cond_expr)?.into_int_value();
        let body_block = ctx.function.append_basic_block(&"WhileBody");
        let end_block = ctx.function.append_basic_block(&"WhileEnd");
        self.builder.build_conditional_branch(cond_value, &body_block, &end_block);
        // WhileBody:
        self.builder.position_at_end(&body_block);
        let rc1 = Rc::new(end_block);
        let rc2 = Rc::clone(&rc1);
        ctx.current_loop_end = Some(rc1);
        self.gen_exprs(ctx, body_exprs)?;
        ctx.current_loop_end = None;
        self.builder.build_unconditional_branch(&begin_block);

        // WhileEnd:
        self.builder.position_at_end(&rc2);
        Ok(self.i32_type.const_int(0, false).as_basic_value_enum()) // return Void
    }

    fn gen_break_expr(&self, 
                      ctx: &mut CodeGenContext) -> Result<inkwell::values::BasicValueEnum, Error> {
        match &ctx.current_loop_end {
            Some(b) => {
                self.builder.build_unconditional_branch(&b);
                Ok(self.i32_type.const_int(0, false).as_basic_value_enum()) // return Void
            },
            None => {
                Err(error::program_error("break outside of a loop"))
            }
        }
    }

    fn gen_lvar_assign(&self,
                       ctx: &mut CodeGenContext,
                       name: &str,
                       rhs: &HirExpression) -> Result<inkwell::values::BasicValueEnum, Error> {
        let value = self.gen_expr(ctx, rhs)?;
        match ctx.lvars.get(name) {
            Some(ptr) => {
                // Reassigning; Just store to it
                self.builder.build_store(*ptr, value);
            },
            None => {
                let ptr = self.builder.build_alloca(self.llvm_type(&rhs.ty), name);
                self.builder.build_store(ptr, value);
                ctx.lvars.insert(name.to_string(), ptr);
            }
        }
        Ok(value)
    }

    fn gen_const_assign(&self,
                        ctx: &mut CodeGenContext,
                        fullname: &ConstFullname,
                        rhs: &HirExpression) -> Result<inkwell::values::BasicValueEnum, Error> {
        let value = self.gen_expr(ctx, rhs)?;
        let ptr = self.module.get_global(&fullname.0).
            expect(&format!("[BUG] global for Constant `{}' not created", fullname.0)).
            as_pointer_value();
        self.builder.build_store(ptr, value);
        Ok(value)
    }

    fn gen_method_call(&self,
                       ctx: &mut CodeGenContext,
                       method_fullname: &MethodFullname,
                       receiver_expr: &HirExpression,
                       arg_exprs: &Vec<HirExpression>) -> Result<inkwell::values::BasicValueEnum, Error> {
        let receiver_value = self.gen_expr(ctx, receiver_expr)?;
        let mut arg_values = arg_exprs.iter().map(|arg_expr|
          self.gen_expr(ctx, arg_expr)
        ).collect::<Result<Vec<_>,_>>()?; // https://github.com/rust-lang/rust/issues/49391

        let function = self.module.get_function(&method_fullname.full_name)
            .expect(&format!("[BUG] get_function not found: {:?}", method_fullname));
        let mut llvm_args = vec!(receiver_value);
        llvm_args.append(&mut arg_values);
        match self.builder.build_call(function, &llvm_args, "result").try_as_basic_value().left() {
            Some(result_value) => Ok(result_value),
            None => {
                // Dummy value (TODO: replace with special value?)
                Ok(self.gen_decimal_literal(42))
            }
        }
    }

    fn gen_lvar_ref(&self,
                    ctx: &mut CodeGenContext,
                    name: &str) -> Result<inkwell::values::BasicValueEnum, Error> {
        let ptr = ctx.lvars.get(name)
            .expect("[BUG] lvar not declared");
        Ok(self.builder.build_load(*ptr, name))
    }

    fn gen_float_literal(&self, value: f64) -> inkwell::values::BasicValueEnum {
        self.f64_type.const_float(value).as_basic_value_enum()
    }

    fn gen_decimal_literal(&self, value: i32) -> inkwell::values::BasicValueEnum {
        self.i32_type.const_int(value as u64, false).as_basic_value_enum()
    }

    fn gen_boolean_literal(&self, value: bool) -> inkwell::values::BasicValueEnum {
        let i = if value { 1 } else { 0 };
        self.i1_type.const_int(i, false).as_basic_value_enum()
    }

    fn gen_class_literal(&self, fullname: &ClassFullname) -> inkwell::values::BasicValueEnum {
        self.allocate_sk_obj(&ty::meta(&fullname.0).fullname)
    }

    // Generate call of GC_malloc and returns a ptr to Shiika object
    pub fn allocate_sk_obj(&self, class_fullname: &ClassFullname) -> inkwell::values::BasicValueEnum {
        let object_type = self.llvm_struct_types.get(&class_fullname).unwrap();
        let obj_ptr_type = object_type.ptr_type(AddressSpace::Generic);
        let size = object_type.size_of()
            .expect("[BUG] object_type has no size");

        // %raw_addr = call i8* @GC_malloc(i64 %size)",
        let func = self.module.get_function("GC_malloc").unwrap();
        let raw_addr = self.builder.build_call(func, &[size.as_basic_value_enum()], "raw_addr").try_as_basic_value().left().unwrap();

        // %addr = bitcast i8* %raw_addr to %#{t}*",
        self.builder.build_bitcast(raw_addr, obj_ptr_type, "addr")
    }

    fn llvm_struct_type(&self, name: &str) -> inkwell::types::StructType {
        let struct_type = self.context.opaque_struct_type(name);
        struct_type.set_body(&[], true); // TODO: ivars
        struct_type
    }

    fn llvm_func_type(&self, self_ty: &TermTy, signature: &MethodSignature) -> inkwell::types::FunctionType {
        let self_type = self.llvm_type(self_ty);
        let mut arg_types = signature.params.iter().map(|param| self.llvm_type(&param.ty)).collect::<Vec<_>>();
        arg_types.insert(0, self_type);

        if signature.ret_ty.is_void_type() {
            self.void_type.fn_type(&arg_types, false)
        }
        else {
            let result_type = self.llvm_type(&signature.ret_ty);
            result_type.fn_type(&arg_types, false)
        }
    }

    fn llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum {
        match ty.body {
            TyBody::TyRaw => {
                match ty.fullname.0.as_str() {
                    "Bool" => self.i1_type.as_basic_type_enum(),
                    "Int" => self.i32_type.as_basic_type_enum(),
                    "Float" => self.f64_type.as_basic_type_enum(),
                    // TODO: replace with special value?
                    "Void" => self.i32_type.as_basic_type_enum(),
                    _ => self.sk_obj_llvm_type(ty)
                }
            },
            _ => self.sk_obj_llvm_type(ty)
        }
    }

    /// Return zero value in LLVM. None if it is a pointer
    fn llvm_zero_value(&self, ty: &TermTy) -> Option<inkwell::values::BasicValueEnum> {
        match ty.body {
            TyBody::TyRaw => {
                match ty.fullname.0.as_str() {
                    "Bool" => Some(self.i1_type.const_int(0, false).as_basic_value_enum()),
                    "Int" => Some(self.i32_type.const_int(0, false).as_basic_value_enum()),
                    "Float" => Some(self.f64_type.const_float(0.0).as_basic_value_enum()),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    fn sk_obj_llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum {
        let struct_type = self.llvm_struct_types.get(&ty.fullname)
            .expect(&format!("[BUG] struct_type not found: {:?}", ty.fullname));
        struct_type.ptr_type(AddressSpace::Generic).as_basic_type_enum()
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
