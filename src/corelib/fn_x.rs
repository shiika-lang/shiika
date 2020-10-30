use crate::corelib::*;
use inkwell::types::*;
use inkwell::AddressSpace;

macro_rules! create_fn_call {
    ($i:expr) => {{
        let args_str = (1..=$i)
            .map(|i| format!("arg{}: S{}", i, i))
            .collect::<Vec<_>>()
            .join(", ");

        let mut typarams = (1..=$i).map(|i| format!("S{}", i)).collect::<Vec<_>>();
        typarams.push("T".to_string());

        create_method_generic(
            &format!("Fn{}", $i),
            &format!("call({}) -> T", args_str),
            |code_gen, function| {
                let fn_obj = function.get_params()[0];
                let sk_ptr = code_gen.build_ivar_load(fn_obj, 0, "func");
                let capary = code_gen.build_ivar_load(fn_obj, 1, "captures");

                let mut args = vec![];
                for k in 1..=$i {
                    args.push(function.get_params()[k]);
                }
                args.push(capary);

                // Create the type of lambda_xx()
                let obj_type = code_gen.llvm_type(&ty::raw("Object"));
                let ary_type = code_gen.llvm_type(&ty::raw("Array"));
                let mut arg_types = vec![];
                for _ in 1..=$i {
                    arg_types.push(obj_type.into());
                }
                arg_types.push(ary_type.into());
                let fntype = obj_type.fn_type(&arg_types, false);
                let fnptype = fntype.ptr_type(AddressSpace::Generic);

                // Cast `fnptr` to that type
                let fnptr = code_gen.unbox_i8ptr(sk_ptr);
                let func = code_gen
                    .builder
                    .build_bitcast(fnptr, fnptype, "")
                    .into_pointer_value();

                // Generate function call
                let result = code_gen
                    .builder
                    .build_call(func, &args, "result")
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                code_gen.builder.build_return(Some(&result));
                Ok(())
            },
            &typarams,
        )
    }};
}

macro_rules! fn_item {
    ($i:expr) => {{
        let mut typarams = (1..=$i).map(|i| format!("S{}", i)).collect::<Vec<_>>();
        typarams.push("T".to_string());

        (
            format!("Fn{}", $i),
            vec![create_fn_call!($i)],
            vec![],
            HashMap::new(),
            typarams,
        )
    }};
}

pub fn fn_items() -> Vec<ClassItem> {
    vec![
        fn_item!(0),
        fn_item!(1),
        fn_item!(2),
        fn_item!(3),
        fn_item!(4),
        fn_item!(5),
        fn_item!(6),
        fn_item!(7),
        fn_item!(8),
        fn_item!(9),
    ]
}
