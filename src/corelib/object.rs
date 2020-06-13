use inkwell::values::*;
use crate::hir::*;
use crate::corelib::create_method;

pub fn create_methods() -> Vec<SkMethod> {
    vec![

    create_method("Object", "initialize() -> Void", |code_gen, _function| {
        code_gen.builder.build_return(None);
        Ok(())
    }),

    create_method("Object", "putchar(ord: Int) -> Void", |code_gen, function| {
        let sk_int = function.get_params()[1];
        let n = code_gen.unbox_int(&sk_int);
        let func = code_gen.module.get_function("putchar").unwrap();
        code_gen.builder.build_call(func, &[n.as_basic_value_enum()], "");
        code_gen.builder.build_return(None);
        Ok(())
    }),

    create_method("Object", "putd(n: Int) -> Void", |code_gen, function| {
        let sk_int = function.get_params()[1];
        let n = code_gen.unbox_int(&sk_int);
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

    create_method("Object", "putf(n: Float) -> Void", |code_gen, function| {
        let arg = function.get_params()[1];
        let n = code_gen.unbox_float(&arg);
        let printf = code_gen.module.get_function("printf").unwrap();
        let tmpl = code_gen.module.get_global("putf_tmpl").unwrap().as_pointer_value();
        let tmpl_ptr = unsafe {
            tmpl.const_in_bounds_gep(&[code_gen.i32_type.const_int(0, false),
                                       code_gen.i32_type.const_int(0, false)])
        };
        code_gen.builder.build_call(printf, &[tmpl_ptr.into(), n.into()], "");
        code_gen.builder.build_return(None);
        Ok(())
    }),

    create_method("Object", "puts(s: String) -> Void", |code_gen, function| {
        let s = function.get_params()[1].into_pointer_value();
        let pptr = code_gen.builder.build_struct_gep(s, 0, "").unwrap();
        let ptr = code_gen.builder.build_load(pptr, "");
        let func = code_gen.module.get_function("puts").unwrap();
        code_gen.builder.build_call(func, &[ptr], "");
        code_gen.builder.build_return(None);
        Ok(())
    }),

    ]
}
