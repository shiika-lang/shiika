//use inkwell::values::*;
use crate::corelib::create_method;
use crate::hir::*;

pub fn create_class_methods() -> Vec<SkMethod> {
    vec![
        create_method(
            "Meta:Shiika::Internal::Memory",
            "gc_malloc(n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let sk_int = code_gen.get_method_param(function, 0);
                let n_bytes = code_gen.unbox_int(sk_int);
                let n_bytes_64 =
                    code_gen
                        .builder
                        .build_int_z_extend(n_bytes, code_gen.i64_type, "n_bytes_64");
                let func = code_gen.module.get_function("GC_malloc").unwrap();
                let mem = code_gen
                    .builder
                    .build_call(func, &[n_bytes_64.into()], "mem")
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let skptr = code_gen.box_i8ptr(mem.into_pointer_value());
                code_gen.builder.build_return(Some(&skptr));
                Ok(())
            },
        ),
        create_method(
            "Meta:Shiika::Internal::Memory",
            "gc_realloc(ptr: Shiika::Internal::Ptr, n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let ptr = code_gen.unbox_i8ptr(code_gen.get_method_param(function, 0));
                let n_bytes = code_gen.unbox_int(code_gen.get_method_param(function, 1));
                let n_bytes_64 =
                    code_gen
                        .builder
                        .build_int_z_extend(n_bytes, code_gen.i64_type, "n_bytes_64");
                let func = code_gen.module.get_function("GC_realloc").unwrap();
                let mem = code_gen
                    .builder
                    .build_call(func, &[ptr.into(), n_bytes_64.into()], "mem")
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let skptr = code_gen.box_i8ptr(mem.into_pointer_value());
                code_gen.builder.build_return(Some(&skptr));
                Ok(())
            },
        ),
        //    create_method("Shiika::Internal::Memory", "memset(ptr: Shiika::Internal::Ptr, n_bytes: Int) -> MutableString", |code_gen, function| {
        //    }),
        //
        //
        create_method(
            "Meta:Shiika::Internal::Memory",
            "memcpy(dst: Shiika::Internal::Ptr, src: Shiika::Internal::Ptr, n_bytes: Int) -> Void",
            |code_gen, function| {
                let dst = code_gen.unbox_i8ptr(code_gen.get_method_param(function, 0));
                let src = code_gen.unbox_i8ptr(code_gen.get_method_param(function, 1));
                let n_bytes = code_gen.unbox_int(code_gen.get_method_param(function, 2));
                let n_bytes_64 =
                    code_gen
                        .builder
                        .build_int_z_extend(n_bytes, code_gen.i64_type, "n_bytes_64");
                let func = code_gen
                    .module
                    .get_function("llvm.memcpy.p0i8.p0i8.i64")
                    .unwrap();
                code_gen.builder.build_call(
                    func,
                    &[
                        dst.into(),
                        src.into(),
                        n_bytes_64.into(),
                        code_gen.i32_type.const_int(0, false).into(),
                        code_gen.i1_type.const_int(0, false).into(),
                    ],
                    "mem",
                );
                code_gen.build_return_void();
                Ok(())
            },
        ),
    ]
}
