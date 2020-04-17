use crate::hir::*;
use crate::stdlib::create_method;

pub fn create_class_methods() -> Vec<SkMethod> {
    vec![

    create_method("Meta:Math", "sin(x: Float) -> Float", |code_gen, function| {
        let x = function.get_params()[1].into_float_value();
        let func = code_gen.module.get_function("sin").unwrap();
        let result = code_gen.builder.build_call(func, &[x.into()], "result").try_as_basic_value().left().unwrap();
        code_gen.builder.build_return(Some(&result));
        Ok(())
    }),

    create_method("Meta:Math", "cos(x: Float) -> Float", |code_gen, function| {
        let x = function.get_params()[1].into_float_value();
        let func = code_gen.module.get_function("cos").unwrap();
        let result = code_gen.builder.build_call(func, &[x.into()], "result").try_as_basic_value().left().unwrap();
        code_gen.builder.build_return(Some(&result));
        Ok(())
    }),

    create_method("Meta:Math", "sqrt(x: Float) -> Float", |code_gen, function| {
        let x = function.get_params()[1].into_float_value();
        let func = code_gen.module.get_function("sqrt").unwrap();
        let result = code_gen.builder.build_call(func, &[x.into()], "result").try_as_basic_value().left().unwrap();
        code_gen.builder.build_return(Some(&result));
        Ok(())
    }),

    ]
}
