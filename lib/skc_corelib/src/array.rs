use crate::create_method_generic;
use shiika_core::ty;
use skc_hir2ll::hir::SkMethod;

pub fn create_methods() -> Vec<SkMethod> {
    // Internal method to downcast Object (return value of Shiika::Internal::Ptr.load) to T, which
    // cannot be done in Shiika level
    vec![create_method_generic(
        "Array",
        "_corelib_array_get(ptr: Shiika::Internal::Ptr) -> T",
        |code_gen, function| {
            let sk_ptr = code_gen.get_nth_param(function, 1);
            let i8ptr = code_gen.unbox_i8ptr(sk_ptr);
            // Object = T's upper bound
            let obj_ptr_type = code_gen.llvm_type(&ty::raw("Object")).into_pointer_type();
            let obj_ptrptr_type = obj_ptr_type.ptr_type(inkwell::AddressSpace::Generic);
            let obj_ptr = code_gen
                .builder
                .build_bitcast(i8ptr.0, obj_ptrptr_type, "")
                .into_pointer_value();
            let loaded = code_gen.builder.build_load(obj_ptr, "element");
            code_gen.builder.build_return(Some(&loaded));
            Ok(())
        },
        &["T".to_string()],
    )]
}
