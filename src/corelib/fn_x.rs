use crate::corelib::*;
use crate::hir::*;
use inkwell::AddressSpace;

pub fn create_methods_1() -> Vec<SkMethod> {
    vec![create_method_generic(
        "Fn1",
        "call(arg1: S1) -> T",
        |code_gen, function| {
            let receiver = function.get_params()[0];
            let args = vec![function.get_params()[1]];
            let ptr = code_gen.build_ivar_load(receiver, 0, "@func");

            let struct_type = code_gen
                .llvm_struct_types
                .get(&class_fullname("Object"))
                .unwrap();
            let obj_type = struct_type.ptr_type(AddressSpace::Generic);
            let fntype = obj_type.fn_type(&[obj_type.into()], false);
            let fnptype = fntype.ptr_type(AddressSpace::Generic);

            let func = code_gen
                .builder
                .build_bitcast(ptr, fnptype, "")
                .into_pointer_value();
            let result = code_gen
                .builder
                .build_call(func, &args, "result")
                .try_as_basic_value()
                .left()
                .unwrap();
            code_gen.builder.build_return(Some(&result));
            Ok(())
        },
        &vec!["S1".to_string(), "T".to_string()],
    )]
}
