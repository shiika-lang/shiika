use crate::code_gen::CodeGen;
use crate::corelib::create_method;
use crate::hir::*;
use inkwell::values::{FloatValue, IntValue};

macro_rules! create_comparison_method {
    ($operator:expr, $body:item) => (
        create_method("Float", format!("{}(other: Float) -> Bool", $operator).as_str(), |code_gen, function| {
            let this = function.get_params()[0];
            let val1 = code_gen.unbox_float(this);
            let that = function.get_params()[1];
            let val2 = code_gen.unbox_float(that);
            $body;
            let result = f(code_gen, val1, val2);
            let sk_result = code_gen.box_bool(&result);
            code_gen.builder.build_return(Some(&sk_result));
            Ok(())
        })
    )
}

macro_rules! create_arithmetic_method {
    ($operator:expr, $body:item) => (
        create_method("Float", format!("{}(other: Float) -> Float", $operator).as_str(), |code_gen, function| {
            let this = function.get_params()[0];
            let val1 = code_gen.unbox_float(this);
            let that = function.get_params()[1];
            let val2 = code_gen.unbox_float(that);
            $body;
            let result = f(code_gen, val1, val2);
            let sk_result = code_gen.box_float(&result);
            code_gen.builder.build_return(Some(&sk_result));
            Ok(())
        })
    )
}

pub fn create_methods() -> Vec<SkMethod> {
    vec![
        create_comparison_method!("==", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> IntValue<'a> {
            code_gen.builder.build_float_compare(inkwell::FloatPredicate::OEQ, val1, val2, "eq")
        }),

        create_comparison_method!("!=", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> IntValue<'a> {
            code_gen.builder.build_float_compare(inkwell::FloatPredicate::UNE, val1, val2, "neq")
        }),

        create_comparison_method!("<", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> IntValue<'a> {
            code_gen.builder.build_float_compare(inkwell::FloatPredicate::OLT, val1, val2, "lt")
        }),

        create_comparison_method!(">", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> IntValue<'a> {
            code_gen.builder.build_float_compare(inkwell::FloatPredicate::OGT, val1, val2, "gt")
        }),

        create_comparison_method!("<=", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> IntValue<'a> {
            code_gen.builder.build_float_compare(inkwell::FloatPredicate::OLE, val1, val2, "leq")
        }),

        create_comparison_method!(">=", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> IntValue<'a> {
            code_gen.builder.build_float_compare(inkwell::FloatPredicate::OGE, val1, val2, "geq")
        }),

        create_arithmetic_method!("+", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> FloatValue<'a> {
            code_gen.builder.build_float_add(val1, val2, "add")
        }),

        create_arithmetic_method!("-", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> FloatValue<'a> {
            code_gen.builder.build_float_sub(val1, val2, "sub")
        }),

        create_arithmetic_method!("*", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> FloatValue<'a> {
            code_gen.builder.build_float_mul(val1, val2, "mul")
        }),

        create_arithmetic_method!("/", fn f<'a>(code_gen: &'a CodeGen, val1: FloatValue<'a>, val2: FloatValue<'a>) -> FloatValue<'a> {
            code_gen.builder.build_float_div(val1, val2, "div")
        }),

        create_method("Float", "abs -> Float", |code_gen, function| {
            let this = function.get_params()[0];
            let x = code_gen.unbox_float(this);
            let func = code_gen.module.get_function("fabs").unwrap();
            let result = code_gen.builder.build_call(func, &[x.into()], "result").try_as_basic_value().left().unwrap();
            let sk_result = code_gen.box_float(&result.into_float_value());
            code_gen.builder.build_return(Some(&sk_result));
            Ok(())
        }),

        create_method("Float", "floor -> Float", |code_gen, function| {
            let this = function.get_params()[0];
            let x = code_gen.unbox_float(this);
            let func = code_gen.module.get_function("floor").unwrap();
            let result = code_gen.builder.build_call(func, &[x.into()], "result").try_as_basic_value().left().unwrap();
            let sk_result = code_gen.box_float(&result.into_float_value());
            code_gen.builder.build_return(Some(&sk_result));
            Ok(())
        }),

        create_method("Float", "to_i() -> Int", |code_gen, function| {
            let this = function.get_params()[0];
            let float = code_gen.unbox_float(this);
            let int = code_gen.builder.build_float_to_signed_int(float, code_gen.i32_type, "int");
            let sk_int = code_gen.box_int(&int);
            code_gen.builder.build_return(Some(&sk_int));
            Ok(())
        }),

        create_method("Float", "-@ -> Float", |code_gen, function| {
            let this = function.get_params()[0];
            let float = code_gen.unbox_float(this);
            let zero = code_gen.f64_type.const_float(0.0);
            let result = code_gen.builder.build_float_sub(zero, float, "result");
            let sk_result = code_gen.box_float(&result);
            code_gen.builder.build_return(Some(&sk_result));
            Ok(())
        }),
    ]
}
