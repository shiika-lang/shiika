//use inkwell::values::*;
use crate::hir::*;
use crate::stdlib::create_method;

pub fn create_class_methods() -> Vec<SkMethod> {
    vec![

    create_method("Meta:Shiika::Internal::Memory", "gc_malloc(n_bytes: Int) -> Shiika::Internal::Ptr", |code_gen, function| {
        let n_bytes = function.get_params()[0];
        let func = code_gen.module.get_function("GC_malloc").unwrap();
        code_gen.builder.build_call(func, &[n_bytes.into()], "mem");
        code_gen.builder.build_return(None);
        Ok(())
    }),

    create_method("Shiika::Internal::Memory", "gc_realloc(ptr: Shiika::Internal::Ptr, n_bytes: Int) -> MutableString", |code_gen, function| {
        let ptr = function.get_params()[0];
        let n_bytes = function.get_params()[1];
        let func = code_gen.module.get_function("GC_realloc").unwrap();
        code_gen.builder.build_call(func, &[ptr.into(), n_bytes.into()], "mem");
        code_gen.builder.build_return(None);
        Ok(())
    }),

//    create_method("Shiika::Internal::Memory", "memset(ptr: Shiika::Internal::Ptr, n_bytes: Int) -> MutableString", |code_gen, function| {
//    }),
//
//    create_method("Shiika::Internal::Memory", "memcpy(src: Shiika::Internal::Ptr, dst: Shiika::Internal::Ptr, n_bytes: Int) -> MutableString", |code_gen, function| {
//    })

    ]
}


