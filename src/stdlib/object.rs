use std::collections::HashMap;
use inkwell::values::*;
use crate::ty;
use crate::hir::*;
use crate::stdlib::define_method;

pub fn create_class() -> SkClass {
    SkClass {
        fullname: "Object".to_string(),
        methods: create_methods(),
    }
}

pub fn create_methods() -> HashMap<String, SkMethod> {
    let mut ret = HashMap::new();

    define_method(&mut ret, "Object", "putchar", vec!(ty::raw("Int")), ty::raw("Void"), |code_gen, function| {
        let n = function.get_params()[1].into_int_value();
        let func = code_gen.module.get_function("putchar").unwrap();
        code_gen.builder.build_call(func, &[n.as_basic_value_enum()], "putchar");
        code_gen.builder.build_return(None);
        Ok(())
    });

    ret
}
