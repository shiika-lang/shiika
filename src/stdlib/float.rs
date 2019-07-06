use crate::ty;
use crate::hir::*;
use crate::stdlib::create_method;

pub fn create_class() -> SkClass {
    SkClass {
        fullname: "Float".to_string(),
        methods: create_methods(),
    }
}

fn create_methods() -> Vec<SkMethod> {
    vec![

    create_method("Float", "+", vec!(ty::raw("Float")), ty::raw("Float"), |code_gen, function| {
        let val1 = function.get_params()[0].into_float_value();
        let val2 = function.get_params()[1].into_float_value();
        let result = code_gen.builder.build_float_add(val1, val2, "result");
        code_gen.builder.build_return(Some(&result));
        Ok(())
    }),

    create_method("Float", "to_i", vec!(ty::raw("Float")), ty::raw("Int"), |code_gen, function| {
        let float = function.get_params()[0].into_float_value();
        let int = code_gen.builder.build_float_to_signed_int(float, code_gen.i32_type, "int");
        code_gen.builder.build_return(Some(&int));
        Ok(())
    }),

    ]
}
