use crate::corelib::create_method;
use crate::hir::*;
use inkwell::values::*;

pub fn create_methods() -> Vec<SkMethod> {
    vec![
        create_method("Object", "initialize() -> Void", |code_gen, _function| {
            code_gen.build_return_void();
            Ok(())
        }),
        create_method("Object", "putd(n: Int) -> Void", |code_gen, function| {
            let sk_int = function.get_params()[1];
            let n = code_gen.unbox_int(sk_int);
            let printf = code_gen.module.get_function("printf").unwrap();
            let tmpl = code_gen
                .module
                .get_global("putd_tmpl")
                .unwrap()
                .as_pointer_value();
            let tmpl_ptr = unsafe {
                tmpl.const_in_bounds_gep(&[
                    code_gen.i32_type.const_int(0, false),
                    code_gen.i32_type.const_int(0, false),
                ])
            };
            code_gen
                .builder
                .build_call(printf, &[tmpl_ptr.into(), n.into()], "");
            code_gen.build_return_void();
            Ok(())
        }),
        create_method("Object", "putf(n: Float) -> Void", |code_gen, function| {
            let arg = function.get_params()[1];
            let n = code_gen.unbox_float(arg);
            let printf = code_gen.module.get_function("printf").unwrap();
            let tmpl = code_gen
                .module
                .get_global("putf_tmpl")
                .unwrap()
                .as_pointer_value();
            let tmpl_ptr = unsafe {
                tmpl.const_in_bounds_gep(&[
                    code_gen.i32_type.const_int(0, false),
                    code_gen.i32_type.const_int(0, false),
                ])
            };
            code_gen
                .builder
                .build_call(printf, &[tmpl_ptr.into(), n.into()], "");
            code_gen.build_return_void();
            Ok(())
        }),
        create_method("Object", "puts(s: String) -> Void", |code_gen, function| {
            let sk_str = function.get_params()[1];
            let sk_ptr = code_gen.build_ivar_load(sk_str, 0, "@sk_ptr");
            let ptr = code_gen.unbox_i8ptr(sk_ptr);
            let func = code_gen.module.get_function("puts").unwrap();
            code_gen.builder.build_call(func, &[ptr.into()], "");
            code_gen.build_return_void();
            Ok(())
        }),
        create_method(
            "Object",
            "exit(status: Int) -> Never",
            |code_gen, function| {
                let sk_int = function.get_params()[1];
                let int64 = code_gen.unbox_int(sk_int);
                let int32 = code_gen
                    .builder
                    .build_int_truncate(int64, code_gen.i32_type, "int32");
                let func = code_gen.module.get_function("exit").unwrap();
                code_gen
                    .builder
                    .build_call(func, &[int32.as_basic_value_enum()], "");
                code_gen.builder.build_return(None);
                Ok(())
            },
        ),
    ]
}
