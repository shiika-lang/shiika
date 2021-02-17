use crate::corelib::create_method;
use crate::hir::*;
use crate::ty;

pub fn create_methods() -> Vec<SkMethod> {
    vec![
        create_method(
            "Shiika::Internal::Ptr",
            "+(n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let ptr = code_gen.unbox_i8ptr(function.get_params()[0]);
                let sk_int = function.get_params()[1];
                let n_bytes = code_gen.unbox_int(sk_int);
                let newptr = unsafe { code_gen.builder.build_gep(ptr, &[n_bytes], "newptr") };
                let skptr = code_gen.box_i8ptr(newptr);
                code_gen.builder.build_return(Some(&skptr));
                Ok(())
            },
        ),
        create_method(
            "Shiika::Internal::Ptr",
            "store(value: Object)",
            |code_gen, function| {
                let i8ptr = code_gen.unbox_i8ptr(function.get_params()[0]);
                let obj_ptr_type = code_gen.llvm_type(&ty::raw("Object")).into_pointer_type();
                let obj_ptrptr_type = obj_ptr_type.ptr_type(inkwell::AddressSpace::Generic);
                let obj_ptr = code_gen
                    .builder
                    .build_bitcast(i8ptr, obj_ptrptr_type, "")
                    .into_pointer_value();
                let sk_obj = function.get_params()[1];
                code_gen.builder.build_store(obj_ptr, sk_obj);
                code_gen.build_return_void();
                Ok(())
            },
        ),
        create_method(
            "Shiika::Internal::Ptr",
            "load -> Object",
            |code_gen, function| {
                let i8ptr = code_gen.unbox_i8ptr(function.get_params()[0]);
                let obj_ptr_type = code_gen.llvm_type(&ty::raw("Object")).into_pointer_type();
                let obj_ptrptr_type = obj_ptr_type.ptr_type(inkwell::AddressSpace::Generic);
                let obj_ptr = code_gen
                    .builder
                    .build_bitcast(i8ptr, obj_ptrptr_type, "")
                    .into_pointer_value();
                let loaded = code_gen.builder.build_load(obj_ptr, "object");
                code_gen.builder.build_return(Some(&loaded));
                Ok(())
            },
        ),
        create_method(
            "Shiika::Internal::Ptr",
            "read -> Int",
            |code_gen, function| {
                let i8ptr = code_gen.unbox_i8ptr(function.get_params()[0]);
                let i8val = code_gen.builder.build_load(i8ptr, "i8val").into_int_value();
                let i64val =
                    code_gen
                        .builder
                        .build_int_z_extend(i8val, code_gen.i64_type, "i64val");
                let sk_int = code_gen.box_int(&i64val);
                code_gen.builder.build_return(Some(&sk_int));
                Ok(())
            },
        ),
        create_method(
            "Shiika::Internal::Ptr",
            "write(byte: Int)",
            |code_gen, function| {
                let i8ptr = code_gen.unbox_i8ptr(function.get_params()[0]);
                let i64val = code_gen.unbox_int(function.get_params()[1]);
                let i8val = code_gen
                    .builder
                    .build_int_truncate(i64val, code_gen.i8_type, "i8val");

                code_gen.builder.build_store(i8ptr, i8val);
                code_gen.build_return_void();
                Ok(())
            },
        ),
    ]
}
