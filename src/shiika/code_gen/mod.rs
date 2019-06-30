//use failure::Fail;
use backtrace::Backtrace;
use inkwell::values::*;
use inkwell::types::*;
use crate::shiika::ty::*;
use crate::shiika::hir::*;
use crate::shiika::hir::HirExpressionBase::*;

#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub backtrace: Backtrace
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for Error {}

pub struct CodeGen {
    context: inkwell::context::Context,
    pub module: inkwell::module::Module,
    builder: inkwell::builder::Builder,
    i32_type: inkwell::types::IntType,
    f32_type: inkwell::types::FloatType,
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
        }
    }

    pub fn gen_program(&self, hir: Hir) -> Result<(), Error> {
        let i32_type = self.i32_type;

        // declare i32 @putchar(i32)
        let putchar_type = i32_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("putchar", putchar_type, None);

        // define i32 @main() {
        let main_type = i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", main_type, None);
        let basic_block = self.context.append_basic_block(&function, "entry");
        self.builder.position_at_end(&basic_block);

        let expr_value = self.gen_expr(function, &hir.hir_expr)?;
        let float_val = 
            match expr_value {
                inkwell::values::BasicValueEnum::FloatValue(v) => v,
                _ => panic!("not float")
            };

        // call i32 @putchar(i32 72)
        let fun = self.module.get_function("putchar");
        // %reg353 = fptosi double 32.0 to i32
        let cast2 = self.builder.build_float_to_signed_int(float_val, self.i32_type, "");
        self.builder.build_call(fun.unwrap(), &[cast2.as_basic_value_enum()], "putchar");
        self.builder.build_call(fun.unwrap(), &[i32_type.const_int(72, false).into()], "putchar");
        self.builder.build_call(fun.unwrap(), &[i32_type.const_int(106, false).into()], "putchar");

        // ret i32 0
        self.builder.build_return(Some(&i32_type.const_int(0, false)));
        Ok(())
    }

    fn gen_expr(&self,
                function: inkwell::values::FunctionValue,
                expr: &HirExpression) -> Result<inkwell::values::BasicValueEnum, Error> {
        match &expr.node {
            HirIfExpression { cond_expr, then_expr, else_expr } => {
                self.gen_if_expr(function, &expr.ty, &cond_expr, &then_expr, &else_expr)
            },
            HirFloatLiteral { value } => {
                Ok(self.gen_float_literal(*value))
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

        let phi_node = self.builder.build_phi(self.llvm_basic_type(ty), "");
        phi_node.add_incoming(&[(then_value, &then_block), (&else_value, &else_block)]);
        Ok(phi_node.as_basic_value())
    }

    //fn gen_method_call

    //fn gen_bin_op
    
    fn gen_float_literal(&self, value: f32) -> inkwell::values::BasicValueEnum {
        self.f32_type.const_float(value as f64).as_basic_value_enum()
    }

    fn gen_decimal_literal(&self, value: i32) -> inkwell::values::BasicValueEnum {
        self.i32_type.const_int(value as u64, false).as_basic_value_enum()
    }

    fn llvm_basic_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum {
        match ty {
            TermTy::TyRaw { fullname } => {
                if fullname == "Float" {
                    return self.f32_type.as_basic_type_enum();
                }
                else {
                    panic!("TODO")
                }
            },
            TermTy::TyMeta { base_fullname } => panic!("TODO")
        }
    }
}
