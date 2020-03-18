use inkwell::values::*;
use crate::hir::*;
use crate::stdlib::create_method;

pub fn create_methods() -> Vec<SkMethod> {
    vec![

// TODO: `new' will mostly look like this, but we need spacial care for `new' because
// 1. its parameters are defined by `initialize', and
// 2. the return type differs in each class
//    create_class_method("Object", "new(params) -> Object", |code_gen, function| {
//        let addr = code_gen.allocate_sk_obj(&ClassFullname("Object".to_string()));
//        code_gen.builder.build_return(Some(addr));
//        Ok(())
//    }),

    create_method("Object", "putchar(ord: Int) -> Void", |code_gen, function| {
        let n = function.get_params()[1].into_int_value();
        let func = code_gen.module.get_function("putchar").unwrap();
        code_gen.builder.build_call(func, &[n.as_basic_value_enum()], "");
        code_gen.builder.build_return(None);
        Ok(())
    }),

    create_method("Object", "putd(n: Int) -> Void", |code_gen, function| {
        let n = function.get_params()[1].into_int_value();
        let printf = code_gen.module.get_function("printf").unwrap();
        let tmpl = code_gen.module.get_global("putd_tmpl").unwrap().as_pointer_value();
        let tmpl_ptr = unsafe {
            tmpl.const_in_bounds_gep(&[code_gen.i32_type.const_int(0, false),
                                       code_gen.i32_type.const_int(0, false)])
        };
        code_gen.builder.build_call(printf, &[tmpl_ptr.into(), n.into()], "");
        code_gen.builder.build_return(None);
        Ok(())
    }),

    ]
}
