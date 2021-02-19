use crate::corelib::*;
use inkwell::types::*;
use inkwell::AddressSpace;

/// Index of @func of FnX
const FN_X_FUNC_IDX: usize = 0;

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
                let exit_status = code_gen.box_int(&code_gen.i64_type.const_int(0 as u64, false));
                let sk_ptr = code_gen.build_ivar_load(fn_obj, FN_X_FUNC_IDX, "@func");

                let mut args = vec![fn_obj, exit_status];
                for k in 1..=$i {
                    args.push(function.get_params()[k]);
                }

                // Create the type of lambda_xx()
                let fn_x_type = code_gen.llvm_type(&ty::raw(&format!("Fn{}", $i)));
                let exit_status_type = code_gen.llvm_type(&ty::raw("Int"));
                let obj_type = code_gen.llvm_type(&ty::raw("Object"));
                let mut arg_types = vec![fn_x_type.into(), exit_status_type.into()];
                for _ in 1..=$i {
                    arg_types.push(obj_type.into());
                }
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
            Some(class_fullname("Fn")),
            vec![create_fn_call!($i)],
            vec![],
            ivars(),
            typarams,
        )
    }};
}

fn ivars() -> HashMap<String, SkIVar> {
    let mut ivars = HashMap::new();
    ivars.insert(
        "@func".to_string(),
        SkIVar {
            name: "@func".to_string(),
            idx: 0,
            ty: ty::raw("Shiika::Internal::Ptr"),
            readonly: true,
        },
    );
    ivars.insert(
        "@the_self".to_string(),
        SkIVar {
            name: "@the_self".to_string(),
            idx: 1,
            ty: ty::raw("Object"),
            readonly: true,
        },
    );
    ivars.insert(
        "@captures".to_string(),
        SkIVar {
            name: "@captures".to_string(),
            idx: 2,
            ty: ty::ary(ty::raw("Shiika::Internal::Ptr")),
            readonly: true,
        },
    );
    ivars.insert(
        "@exit_status".to_string(),
        SkIVar {
            name: "@exit_status".to_string(),
            idx: 3,
            ty: ty::raw("Int"),
            readonly: false,
        },
    );
    ivars
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
