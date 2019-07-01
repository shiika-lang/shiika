use crate::shiika::ty;
use crate::shiika::ty::*;
use crate::shiika::hir::*;
use crate::shiika::hir::SkMethodBody::*;

pub fn stdlib_methods() -> Vec<SkMethod> {
    vec!(
        float_add()
    )
}

fn float_add() -> SkMethod {
    SkMethod {
        fullname: "Float#+".to_string(),
        signature: MethodSignature {
            ret_ty: ty::raw("Float"),
            arg_tys: vec!(ty::raw("Float")),
        },
        body: RustMethodBody {
            gen: (|code_gen, function| {
                let basic_block = code_gen.context.append_basic_block(&function, "entry");
                code_gen.builder.position_at_end(&basic_block);

                let val1 = function.get_params()[0].into_float_value();
                let val2 = function.get_params()[1].into_float_value();
                let result = code_gen.builder.build_float_add(val1, val2, "result");
                code_gen.builder.build_return(Some(&result));
                Ok(())
            })
        }
    }
}
