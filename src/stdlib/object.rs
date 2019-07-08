use inkwell::values::*;
use crate::hir::*;
use crate::stdlib::create_method;

pub fn create_class() -> SkClass {
    SkClass {
        fullname: "Object".to_string(),
        methods: create_methods(),
    }
}

pub fn create_methods() -> Vec<SkMethod> {
    vec![

    create_method("Object", "putchar(ord: Int) -> Void", |code_gen, function| {
        let n = function.get_params()[1].into_int_value();
        let func = code_gen.module.get_function("putchar").unwrap();
        code_gen.builder.build_call(func, &[n.as_basic_value_enum()], "putchar");
        code_gen.builder.build_return(None);
        Ok(())
    }),

    ]
}
