use crate::hir::*;
use crate::stdlib::create_method;

pub fn create_methods() -> Vec<SkMethod> {
    vec![

    create_method("Float", "+(other: Float) -> Float", |code_gen, function| {
        let val1 = function.get_params()[0].into_float_value();
        let val2 = function.get_params()[1].into_float_value();
        let result = code_gen.builder.build_float_add(val1, val2, "result");
        code_gen.builder.build_return(Some(&result));
        Ok(())
    }),

    create_method("Float", "abs -> Float", |code_gen, function| {
        let x = function.get_params()[0].into_float_value();
        let func = code_gen.module.get_function("fabs").unwrap();
        let result = code_gen.builder.build_call(func, &[x.into()], "result").try_as_basic_value().left().unwrap();
        code_gen.builder.build_return(Some(&result));
        Ok(())
    }),

    create_method("Float", "to_i() -> Int", |code_gen, function| {
        let float = function.get_params()[0].into_float_value();
        let int = code_gen.builder.build_float_to_signed_int(float, code_gen.i32_type, "int");
        code_gen.builder.build_return(Some(&int));
        Ok(())
    }),

    ]
}
