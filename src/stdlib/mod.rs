mod float;
mod int;
mod object;
mod void;
use crate::hir::*;

pub fn create_classes() -> Vec<SkClass> {
    vec![
        float::create_class(),
        int::create_class(),
        object::create_class(),
        void::create_class(),
    ]
}

pub fn create_method(class_name: &str,
                      sig_str: &str,
                      gen: GenMethodBody) -> SkMethod {
    let mut parser = crate::parser::Parser::new(sig_str);
    let ast_sig = parser.parse_method_signature().unwrap();
    let sig = crate::hir::create_signature(class_name.to_string(), &ast_sig);

    SkMethod {
        signature: sig,
        body: Some(SkMethodBody::RustMethodBody{ gen: gen })
    }
}
