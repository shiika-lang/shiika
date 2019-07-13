use std::collections::HashMap;
use inkwell::AddressSpace;
use inkwell::values::*;
use inkwell::types::*;
use crate::error::Error;
use crate::ty::*;
use crate::hir::*;
use crate::hir::HirExpressionBase::*;
use crate::names::*;

pub struct CodeGen {
    pub context: inkwell::context::Context,
    pub module: inkwell::module::Module,
    pub builder: inkwell::builder::Builder,
    pub i32_type: inkwell::types::IntType,
    pub i64_type: inkwell::types::IntType,
    pub f32_type: inkwell::types::FloatType,
    pub void_type: inkwell::types::VoidType,
    llvm_struct_types: HashMap<ClassFullname, inkwell::types::StructType>,
    // TODO: Remove this after `self` is properly handled
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
            i32_type: inkwell::types::IntType::i32_type(),
            i64_type: inkwell::types::IntType::i64_type(),
            f32_type: inkwell::types::FloatType::f32_type(),
            void_type: inkwell::types::VoidType::void_type(),
            llvm_struct_types: HashMap::new(),
            the_main: None,
        }
    }

    pub fn gen_program(&mut self, hir: Hir, stdlib: &Vec<SkClass>) -> Result<(), Error> {
        self.gen_declares();
        self.gen_classes(stdlib)?;
        self.gen_classes(&hir.sk_classes)?;
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
        self.gen_exprs(function, &main_exprs)?;

        // ret i32 0
        self.builder.build_return(Some(&self.i32_type.const_int(0, false)));
        Ok(())
    }

    fn gen_classes(&mut self, classes: &Vec<SkClass>) -> Result<(), Error> {
        // Create llvm struct types
        classes.iter().for_each(|sk_class| {
            let struct_type = self.context.opaque_struct_type(&sk_class.fullname.0);
            struct_type.set_body(&[], true);
            self.llvm_struct_types.insert(sk_class.fullname.clone(), struct_type);
        });

        // Compile methods
        classes.iter().try_for_each(|sk_class| {
            sk_class.methods.iter().try_for_each(|method| {
                self.gen_method(&sk_class, &method)
            })
        })
    }

    fn gen_method(&self, sk_class: &SkClass, method: &SkMethod) -> Result<(), Error> {
        let func_type = self.llvm_func_type(&sk_class.instance_ty(), &method.signature);
        let function = self.module.add_function(&method.signature.fullname.0, func_type, None);
        let basic_block = self.context.append_basic_block(&function, "");
        self.builder.position_at_end(&basic_block);

        match &method.body {
            SkMethodBody::RustMethodBody { gen } => {
                gen(self, &function)?
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
        let last_value_opt = self.gen_exprs(function, exprs)?;
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
                function: inkwell::values::FunctionValue,
                exprs: &HirExpressions) -> Result<Option<inkwell::values::BasicValueEnum>, Error> {
        let mut last_value_opt = None;
        exprs.exprs.iter().try_for_each(|expr| {
            let value: inkwell::values::BasicValueEnum = self.gen_expr(function, &expr)?;
            last_value_opt = Some(value);
            Ok(())
        })?;
        Ok(last_value_opt)
    }

    fn gen_expr(&self,
                function: inkwell::values::FunctionValue,
                expr: &HirExpression) -> Result<inkwell::values::BasicValueEnum, Error> {
        match &expr.node {
            HirIfExpression { cond_expr, then_expr, else_expr } => {
                self.gen_if_expr(function, &expr.ty, &cond_expr, &then_expr, &else_expr)
            },
            HirMethodCall { receiver_expr, method_fullname, arg_exprs } => {
                self.gen_method_call(function, method_fullname, receiver_expr, arg_exprs)
            },
            HirArgRef { idx } => {
                Ok(function.get_nth_param(*idx + 1).unwrap()) // +1 for the first %self 
            },
            HirSelfExpression => {
                // TODO: get the 0-th param except toplevel
                Ok(self.the_main.unwrap())
            },
            HirFloatLiteral { value } => {
                Ok(self.gen_float_literal(*value))
            },
            HirDecimalLiteral { value } => {
                Ok(self.gen_decimal_literal(*value))
            },
            HirNop => {
                panic!("HirNop not handled by `else`")
            }
        }
    }

    fn gen_if_expr(&self, 
                   function: inkwell::values::FunctionValue,
                   ty: &TermTy,
                   cond_expr: &HirExpression,
                   then_expr: &HirExpression,
                   else_expr: &HirExpression) -> Result<inkwell::values::BasicValueEnum, Error> {
        let cond_value = self.gen_expr(function, cond_expr)?.into_int_value();
        let then_value: &inkwell::values::BasicValue = &self.gen_expr(function, then_expr)?;
        let else_value = self.gen_expr(function, else_expr)?;

        let then_block = function.append_basic_block(&"then");
        let else_block = function.append_basic_block(&"else");
        let merge_block = function.append_basic_block(&"merge");

        self.builder.build_conditional_branch(cond_value, &then_block, &else_block);
        self.builder.position_at_end(&then_block);
        self.builder.build_unconditional_branch(&merge_block);
        let then_block = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(&else_block);
        self.builder.build_unconditional_branch(&merge_block);
        let else_block = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(&merge_block);

        let phi_node = self.builder.build_phi(self.llvm_type(ty), "");
        phi_node.add_incoming(&[(then_value, &then_block), (&else_value, &else_block)]);
        Ok(phi_node.as_basic_value())
    }

    fn gen_method_call(&self,
                       function: inkwell::values::FunctionValue,
                       method_fullname: &MethodFullname,
                       receiver_expr: &HirExpression,
                       arg_exprs: &Vec<HirExpression>) -> Result<inkwell::values::BasicValueEnum, Error> {
        let receiver_value = self.gen_expr(function, receiver_expr)?;
        let mut arg_values = arg_exprs.iter().map(|arg_expr|
          self.gen_expr(function, arg_expr)
        ).collect::<Result<Vec<_>,_>>()?; // https://github.com/rust-lang/rust/issues/49391

        let function = self.module.get_function(&method_fullname.0).expect("[BUG] get_function not found");
        let mut llvm_args = vec!(receiver_value);
        llvm_args.append(&mut arg_values);
        match self.builder.build_call(function, &llvm_args, "gen_method_call").try_as_basic_value().left() {
            Some(result_value) => Ok(result_value),
            None => {
                // Dummy value (TODO: replace with special value?)
                Ok(self.gen_decimal_literal(42))
            }
        }
    }

    fn gen_float_literal(&self, value: f32) -> inkwell::values::BasicValueEnum {
        self.f32_type.const_float(value as f64).as_basic_value_enum()
    }

    fn gen_decimal_literal(&self, value: i32) -> inkwell::values::BasicValueEnum {
        self.i32_type.const_int(value as u64, false).as_basic_value_enum()
    }

    // Generate call of GC_malloc and returns a ptr to Shiika object
    fn allocate_sk_obj(&self, class_fullname: &ClassFullname) -> inkwell::values::BasicValueEnum {
        let object_type = self.llvm_struct_types.get(&class_fullname).unwrap();

        // %size = ptrtoint %#{t}* getelementptr (%#{t}, %#{t}* null, i32 1) to i64",
        let obj_ptr_type = object_type.ptr_type(AddressSpace::Generic);
        let gep = unsafe {
            self.builder.build_in_bounds_gep(
              obj_ptr_type.const_null(),
              &[self.i64_type.const_int(1, false)],
              "",
            )
        };
        let size = self.builder.build_ptr_to_int(gep, self.i64_type, "size");

        // %raw_addr = call i8* @GC_malloc(i64 %size)",
        let func = self.module.get_function("GC_malloc").unwrap();
        let raw_addr = self.builder.build_call(func, &[size.as_basic_value_enum()], "raw_addr").try_as_basic_value().left().unwrap();

        // %addr = bitcast i8* %raw_addr to %#{t}*",
        self.builder.build_bitcast(raw_addr, obj_ptr_type, "addr")
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
                    "Int" => self.i32_type.as_basic_type_enum(),
                    "Float" => self.f32_type.as_basic_type_enum(),
                    // TODO: replace with special value?
                    "Void" => self.i32_type.as_basic_type_enum(),
                    _ => {
                        let struct_type = self.llvm_struct_types.get(&ty.fullname).unwrap();
                        struct_type.ptr_type(AddressSpace::Generic).as_basic_type_enum()
                    }
                }
            },
            TyBody::TyMeta { .. } => panic!("TODO")
        }
    }
}
