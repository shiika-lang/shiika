//use inkwell::values::*;
use crate::corelib::create_method;
use crate::hir::*;

pub fn create_class_methods() -> Vec<SkMethod> {
    vec![
        create_method(
            "Meta:Shiika::Internal::Memory",
            "gc_malloc(n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let sk_int = function.get_params()[1];
                let n_bytes = code_gen.unbox_int(&sk_int);
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
                code_gen.builder.build_return(Some(&mem));
                Ok(())
            },
        ),
        create_method(
            "Meta:Shiika::Internal::Memory",
            "gc_realloc(ptr: Shiika::Internal::Ptr, n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let ptr = function.get_params()[1];
                let sk_int = function.get_params()[2];
                let n_bytes = code_gen.unbox_int(&sk_int);
                let n_bytes_64 =
                    code_gen
                        .builder
                        .build_int_z_extend(n_bytes, code_gen.i64_type, "n_bytes_64");
                let func = code_gen.module.get_function("GC_realloc").unwrap();
                let mem = code_gen
                    .builder
                    .build_call(func, &[ptr, n_bytes_64.into()], "mem")
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                code_gen.builder.build_return(Some(&mem));
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
                let dst = function.get_params()[1];
                let src = function.get_params()[2];
                let sk_int = function.get_params()[3];
                let n_bytes = code_gen.unbox_int(&sk_int);
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
                        dst,
                        src,
                        n_bytes_64.into(),
                        code_gen.i32_type.const_int(0, false).into(),
                        code_gen.i1_type.const_int(0, false).into(),
                    ],
                    "mem",
                );
                code_gen.builder.build_return(None);
                Ok(())
            },
        ),
    ]
}
