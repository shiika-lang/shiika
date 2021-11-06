use crate::create_method;
use inkwell::values::*;
use skc_hir2ll::hir::SkMethod;

pub fn create_methods() -> Vec<SkMethod> {
    vec![
        create_method(
            "Object",
            "==(other: Object) -> Bool",
            |code_gen, function| {
                let receiver = function
                    .get_nth_param(0)
                    .unwrap()
                    .into_pointer_value()
                    .const_to_int(code_gen.i64_type);
                let other = function
                    .get_nth_param(1)
                    .unwrap()
                    .into_pointer_value()
                    .const_to_int(code_gen.i64_type);
                let result = code_gen.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    receiver,
                    other,
                    "eq",
                );
                code_gen.build_return(&code_gen.box_bool(result));
                Ok(())
            },
        ),
        create_method("Object", "initialize() -> Void", |code_gen, _function| {
            code_gen.build_return_void();
            Ok(())
        }),
        create_method("Object", "class() -> Class", |code_gen, function| {
            let receiver = code_gen.get_nth_param(function, 0);
            let cls_obj = code_gen.get_class_of_obj(receiver);
            code_gen.build_return(&cls_obj.as_sk_obj());
            Ok(())
        }),
        create_method("Object", "putd(n: Int) -> Void", |code_gen, function| {
            let n = code_gen.unbox_int(code_gen.get_nth_param(function, 1));
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
            let n = code_gen.unbox_float(code_gen.get_nth_param(function, 1));
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
            let s = code_gen
                .builder
                .build_bitcast(sk_str, code_gen.i8ptr_type, "");
            let func = code_gen.module.get_function("shiika_puts").unwrap();
            code_gen.builder.build_call(func, &[s], "");
            code_gen.build_return_void();
            Ok(())
        }),
        create_method(
            "Object",
            "exit(status: Int) -> Never",
            |code_gen, function| {
                let int64 = code_gen.unbox_int(code_gen.get_nth_param(function, 1));
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
