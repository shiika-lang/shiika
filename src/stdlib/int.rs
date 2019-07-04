use std::collections::HashMap;
use crate::shiika::ty;
use crate::shiika::hir::*;
use crate::shiika::stdlib::define_method;

pub fn create_class() -> SkClass {
    SkClass {
        fullname: "Int".to_string(),
        methods: create_methods(),
    }
}

fn create_methods() -> HashMap<String, SkMethod> {
    let mut ret = HashMap::new();

    define_method(&mut ret, "Int", "+", vec!(ty::raw("Int")), ty::raw("Int"), |code_gen, function| {
        let val1 = function.get_params()[0].into_int_value();
        let val2 = function.get_params()[1].into_int_value();
        let result = code_gen.builder.build_int_add(val1, val2, "result");
        code_gen.builder.build_return(Some(&result));
        Ok(())
    });

    define_method(&mut ret, "Int", "to_f", vec!(ty::raw("Int")), ty::raw("Float"), |code_gen, function| {
        let int = function.get_params()[0].into_int_value();
        let float = code_gen.builder.build_signed_int_to_float(int, code_gen.f32_type, "float");
        code_gen.builder.build_return(Some(&float));
        Ok(())
    });

    ret
}

