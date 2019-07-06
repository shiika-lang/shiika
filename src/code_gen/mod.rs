use inkwell::values::*;
use inkwell::types::*;
use crate::error::Error;
use crate::ty::*;
use crate::hir::*;
use crate::hir::HirExpressionBase::*;

pub struct CodeGen {
    pub context: inkwell::context::Context,
    pub module: inkwell::module::Module,
    pub builder: inkwell::builder::Builder,
    pub i32_type: inkwell::types::IntType,
    pub f32_type: inkwell::types::FloatType,
    pub void_type: inkwell::types::VoidType,
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
            f32_type: inkwell::types::FloatType::f32_type(),
            void_type: inkwell::types::VoidType::void_type(),
        }
    }

    pub fn gen_program(&self, hir: Hir, stdlib: &Vec<SkClass>) -> Result<(), Error> {
        let i32_type = self.i32_type;

        // declare i32 @putchar(i32)
        let putchar_type = i32_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("putchar", putchar_type, None);

        self.gen_stdlib(stdlib)?;

        // define i32 @main() {
        let main_type = i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", main_type, None);
        let basic_block = self.context.append_basic_block(&function, "entry");
        self.builder.position_at_end(&basic_block);

        self.gen_stmts(function, &hir.main_stmts)?;

        // ret i32 0
        self.builder.build_return(Some(&i32_type.const_int(0, false)));
        Ok(())
    }

    fn gen_stdlib(&self, stdlib: &Vec<SkClass>) -> Result<(), Error> {
        stdlib.iter().try_for_each(|sk_class| {
            sk_class.methods.iter().try_for_each(|method| {
                self.gen_method(&sk_class, &method)
            })
        })
    }

    fn gen_method(&self, sk_class: &SkClass, method: &SkMethod) -> Result<(), Error> {
        let func_type = self.llvm_func_type(&sk_class.instance_ty(), &method.signature);
        let function = self.module.add_function(&method.signature.fullname, func_type, None);
        let basic_block = self.context.append_basic_block(&function, "");
        self.builder.position_at_end(&basic_block);

        // Method should have a body in code gen phase
        let body = method.body.as_ref().unwrap();
        match body {
            SkMethodBody::RustMethodBody { gen } => {
                gen(self, &function)?
            },
            //SkMethod::ShiikaMethod =>
            _ => panic!("TODO"),
        }
        Ok(())
    }

    fn gen_stmts(&self,
                function: inkwell::values::FunctionValue,
                stmts: &Vec<HirStatement>) -> Result<(), Error> {
        stmts.iter().try_for_each(|stmt| self.gen_stmt(function, &stmt))
    }

    fn gen_stmt(&self,
                function: inkwell::values::FunctionValue,
                stmt: &HirStatement) -> Result<(), Error> {
        match stmt {
            HirStatement::HirExpressionStatement { expr } => {
                self.gen_expr(function, &expr)?;
                Ok(())
            }
        }
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
            HirSelfExpression => {
                // TODO: generate current self
                Ok(self.gen_decimal_literal(1042))
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
                       method_fullname: &str,
                       receiver_expr: &HirExpression,
                       arg_exprs: &Vec<HirExpression>) -> Result<inkwell::values::BasicValueEnum, Error> {
        let receiver_value = self.gen_expr(function, receiver_expr)?;
        let mut arg_values = arg_exprs.iter().map(|arg_expr|
          self.gen_expr(function, arg_expr)
        ).collect::<Result<Vec<_>,_>>()?; // https://github.com/rust-lang/rust/issues/49391

        let function = self.module.get_function(method_fullname).expect("[BUG] get_function not found");
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

    //fn gen_bin_op

    fn gen_float_literal(&self, value: f32) -> inkwell::values::BasicValueEnum {
        self.f32_type.const_float(value as f64).as_basic_value_enum()
    }

    fn gen_decimal_literal(&self, value: i32) -> inkwell::values::BasicValueEnum {
        self.i32_type.const_int(value as u64, false).as_basic_value_enum()
    }

    fn llvm_func_type(&self, self_ty: &TermTy, signature: &MethodSignature) -> inkwell::types::FunctionType {
        let self_type = self.llvm_type(self_ty);
        let mut arg_types = signature.arg_tys.iter().map(|ty| self.llvm_type(ty)).collect::<Vec<_>>();
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
        match ty {
            TermTy::TyRaw { fullname } => {
                match fullname.as_str() {
                    "Int" => self.i32_type.as_basic_type_enum(),
                    "Float" => self.f32_type.as_basic_type_enum(),
                    // TODO: should be a struct
                    "Object" => self.i32_type.as_basic_type_enum(),
                    // TODO: replace with special value?
                    "Void" => self.i32_type.as_basic_type_enum(),
                    _ => panic!("TODO: {:?}", fullname)
                }
            },
            TermTy::TyMeta { .. } => panic!("TODO")
        }
    }
}
