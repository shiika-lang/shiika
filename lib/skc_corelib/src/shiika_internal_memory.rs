use crate::create_method;
use skc_hir::SkMethod;

pub fn create_class_methods() -> Vec<SkMethod> {
    vec![
        create_method(
            "Meta:Shiika::Internal::Memory",
            "gc_malloc(n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let n_bytes = code_gen.unbox_int(code_gen.get_nth_param(function, 1));
                let n_bytes_64 =
                    code_gen
                        .builder
                        .build_int_z_extend(n_bytes, code_gen.i64_type, "n_bytes_64");
                let mem = code_gen.call_llvm_func("shiika_malloc", &[n_bytes_64.into()], "mem");
                let skptr = code_gen.box_i8ptr(mem);
                code_gen.build_return(&skptr);
                Ok(())
            },
        ),
        create_method(
            "Meta:Shiika::Internal::Memory",
            "gc_realloc(ptr: Shiika::Internal::Ptr, n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let ptr = code_gen.unbox_i8ptr(code_gen.get_nth_param(function, 1));
                let n_bytes = code_gen.unbox_int(code_gen.get_nth_param(function, 2));
                let n_bytes_64 =
                    code_gen
                        .builder
                        .build_int_z_extend(n_bytes, code_gen.i64_type, "n_bytes_64");
                let mem = code_gen.call_llvm_func(
                    "shiika_realloc",
                    &[ptr.0.into(), n_bytes_64.into()],
                    "mem",
                );
                let skptr = code_gen.box_i8ptr(mem);
                code_gen.build_return(&skptr);
                Ok(())
            },
        ),
        create_method(
            "Meta:Shiika::Internal::Memory",
            "memcpy(dst: Shiika::Internal::Ptr, src: Shiika::Internal::Ptr, n_bytes: Int) -> Void",
            |code_gen, function| {
                let dst = code_gen.unbox_i8ptr(code_gen.get_nth_param(function, 1));
                let src = code_gen.unbox_i8ptr(code_gen.get_nth_param(function, 2));
                let n_bytes = code_gen.unbox_int(code_gen.get_nth_param(function, 3));
                let n_bytes_64 =
                    code_gen
                        .builder
                        .build_int_z_extend(n_bytes, code_gen.i64_type, "n_bytes_64");
                code_gen.call_llvm_void_func(
                    "llvm.memcpy.p0i8.p0i8.i64",
                    &[
                        dst.0.into(),
                        src.0.into(),
                        n_bytes_64.into(),
                        code_gen.i32_type.const_int(0, false).into(),
                        code_gen.i1_type.const_int(0, false).into(),
                    ],
                );
                code_gen.build_return_void();
                Ok(())
            },
        ),
    ]
}
