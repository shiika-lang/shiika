//use inkwell::values::*;
use crate::corelib::create_method;
use crate::hir::*;
use crate::ty;

pub fn create_methods() -> Vec<SkMethod> {
    vec![
        create_method(
            "Shiika::Internal::Ptr",
            "+(n_bytes: Int) -> Shiika::Internal::Ptr",
            |code_gen, function| {
                let ptr = function.get_params()[0];
                let sk_int = function.get_params()[1];
                let n_bytes = code_gen.unbox_int(&sk_int);
                let newptr = unsafe {
                    code_gen
                        .builder
                        .build_gep(ptr.into_pointer_value(), &[n_bytes], "newptr")
                };
                code_gen.builder.build_return(Some(&newptr));
                Ok(())
            },
        ),
        create_method(
            "Shiika::Internal::Ptr",
            "store(value: Object)",
            |code_gen, function| {
                let i8ptr = function.get_params()[0].into_pointer_value();

                let obj_ptr_type = code_gen.llvm_type(&ty::raw("Object")).into_pointer_type();
                let obj_ptrptr_type = obj_ptr_type.ptr_type(inkwell::AddressSpace::Generic);
                let obj_ptr = code_gen
                    .builder
                    .build_bitcast(i8ptr, obj_ptrptr_type, "")
                    .into_pointer_value();
                let sk_obj = function.get_params()[1];
                code_gen.builder.build_store(obj_ptr, sk_obj);
                code_gen.builder.build_return(None);
                Ok(())
            },
        ),
        create_method(
            "Shiika::Internal::Ptr",
            "load -> Object",
            |code_gen, function| {
                let i8ptr = function.get_params()[0].into_pointer_value();
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
    ]
}
