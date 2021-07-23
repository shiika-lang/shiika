use crate::corelib::create_method;
use crate::hir::*;

pub fn create_class_methods() -> Vec<SkMethod> {
    vec![
        create_method(
            "Meta:Math",
            "sin(x: Float) -> Float",
            |code_gen, function| {
                let x = code_gen.unbox_float(code_gen.get_nth_param(function, 1));
                let func = code_gen.module.get_function("sin").unwrap();
                let result = code_gen
                    .builder
                    .build_call(func, &[x.into()], "result")
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let sk_result = code_gen.box_float(&result.into_float_value());
                code_gen.build_return(&sk_result);
                Ok(())
            },
        ),
        create_method(
            "Meta:Math",
            "cos(x: Float) -> Float",
            |code_gen, function| {
                let x = code_gen.unbox_float(code_gen.get_nth_param(function, 1));
                let func = code_gen.module.get_function("cos").unwrap();
                let result = code_gen
                    .builder
                    .build_call(func, &[x.into()], "result")
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let sk_result = code_gen.box_float(&result.into_float_value());
                code_gen.build_return(&sk_result);
                Ok(())
            },
        ),
        create_method(
            "Meta:Math",
            "sqrt(x: Float) -> Float",
            |code_gen, function| {
                let x = code_gen.unbox_float(code_gen.get_nth_param(function, 1));
                let func = code_gen.module.get_function("sqrt").unwrap();
                let result = code_gen
                    .builder
                    .build_call(func, &[x.into()], "result")
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let sk_result = code_gen.box_float(&result.into_float_value());
                code_gen.build_return(&sk_result);
                Ok(())
            },
        ),
    ]
}
