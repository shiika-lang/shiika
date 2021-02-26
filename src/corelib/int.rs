use crate::code_gen::CodeGen;
use crate::corelib::create_method;
use crate::hir::*;
use inkwell::values::IntValue;

macro_rules! create_comparison_method {
    ($operator:expr, $body:item) => {
        create_method(
            "Int",
            format!("{}(other: Int) -> Bool", $operator).as_str(),
            |code_gen, function| {
                let val1 = code_gen.unbox_int(code_gen.get_method_receiver(function));
                let val2 = code_gen.unbox_int(code_gen.get_method_param(function, 0));
                $body;
                let result = f(code_gen, val1, val2);
                let sk_result = code_gen.box_bool(result);
                code_gen.builder.build_return(Some(&sk_result));
                Ok(())
            },
        )
    };
}

macro_rules! create_arithmetic_method {
    ($operator:expr, $body:item) => {
        create_method(
            "Int",
            format!("{}(other: Int) -> Int", $operator).as_str(),
            |code_gen, function| {
                let val1 = code_gen.unbox_int(code_gen.get_method_receiver(function));
                let val2 = code_gen.unbox_int(code_gen.get_method_param(function, 0));
                $body;
                let result = f(code_gen, val1, val2);
                let sk_result = code_gen.box_int(&result);
                code_gen.builder.build_return(Some(&sk_result));
                Ok(())
            },
        )
    };
}

pub fn create_methods() -> Vec<SkMethod> {
    vec![
        create_comparison_method!(
            "==",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen
                    .builder
                    .build_int_compare(inkwell::IntPredicate::EQ, val1, val2, "eq")
            }
        ),
        create_comparison_method!(
            "!=",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen
                    .builder
                    .build_int_compare(inkwell::IntPredicate::NE, val1, val2, "neq")
            }
        ),
        create_comparison_method!(
            "<",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen
                    .builder
                    .build_int_compare(inkwell::IntPredicate::SLT, val1, val2, "lt")
            }
        ),
        create_comparison_method!(
            ">",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen
                    .builder
                    .build_int_compare(inkwell::IntPredicate::SGT, val1, val2, "gt")
            }
        ),
        create_comparison_method!(
            "<=",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen
                    .builder
                    .build_int_compare(inkwell::IntPredicate::SLE, val1, val2, "leq")
            }
        ),
        create_comparison_method!(
            ">=",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen
                    .builder
                    .build_int_compare(inkwell::IntPredicate::SGE, val1, val2, "geq")
            }
        ),
        create_arithmetic_method!(
            "+",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_int_add(val1, val2, "add")
            }
        ),
        create_arithmetic_method!(
            "-",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_int_sub(val1, val2, "sub")
            }
        ),
        create_arithmetic_method!(
            "*",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_int_mul(val1, val2, "mul")
            }
        ),
        create_arithmetic_method!(
            "/",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_int_signed_div(val1, val2, "div")
            }
        ),
        create_arithmetic_method!(
            "reminder",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_int_signed_rem(val1, val2, "rem")
            }
        ),
        create_arithmetic_method!(
            "&",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_and(val1, val2, "and")
            }
        ),
        create_arithmetic_method!(
            "|",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_or(val1, val2, "or")
            }
        ),
        create_arithmetic_method!(
            "^",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_xor(val1, val2, "xor")
            }
        ),
        create_arithmetic_method!(
            "<<",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen.builder.build_left_shift(val1, val2, "lshift")
            }
        ),
        create_arithmetic_method!(
            ">>",
            fn f<'a>(
                code_gen: &'a CodeGen,
                val1: IntValue<'a>,
                val2: IntValue<'a>,
            ) -> IntValue<'a> {
                code_gen
                    .builder
                    .build_right_shift(val1, val2, true, "rshift")
            }
        ),
        create_method("Int", "to_f() -> Float", |code_gen, function| {
            let int = code_gen.unbox_int(code_gen.get_method_receiver(function));
            let float = code_gen
                .builder
                .build_signed_int_to_float(int, code_gen.f64_type, "float");
            let sk_result = code_gen.box_float(&float);
            code_gen.builder.build_return(Some(&sk_result));
            Ok(())
        }),
        create_method("Int", "-@ -> Int", |code_gen, function| {
            let this = code_gen.unbox_int(code_gen.get_method_receiver(function));
            let zero = code_gen.i64_type.const_int(0, false);
            let result = code_gen.builder.build_int_sub(zero, this, "result");
            let sk_result = code_gen.box_int(&result);
            code_gen.builder.build_return(Some(&sk_result));
            Ok(())
        }),
    ]
}
