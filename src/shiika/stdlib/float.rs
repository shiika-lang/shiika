use crate::shiika::ty;
use crate::shiika::hir::*;
use crate::shiika::stdlib::define_method;

pub fn define_methods(methods: &mut Vec<SkMethod>) {
    let mut v = vec!(
        define_method("Float#+", vec!(ty::raw("Float")), ty::raw("Float"), |code_gen, function| {
            let val1 = function.get_params()[0].into_float_value();
            let val2 = function.get_params()[1].into_float_value();
            let result = code_gen.builder.build_float_add(val1, val2, "result");
            code_gen.builder.build_return(Some(&result));
            Ok(())
        })
    );
    methods.append(&mut v)
}
