mod float;
mod int;
mod object;
mod void;
use crate::hir::*;

pub fn create_classes() -> Vec<SkClass> {
    let mut v = vec![];
    vec![
        float::create_class(),
        int::create_class(),
        object::create_class(),
        void::create_class(),
    ].iter_mut().for_each(|mut a| v.append(&mut a));
    v
}

pub fn create_method(class_name: &str,
                      sig_str: &str,
                      gen: GenMethodBody) -> SkMethod {
    let mut parser = crate::parser::Parser::new(sig_str);
    let (ast_sig, _) = parser.parse_method_signature().unwrap();
    let sig = crate::hir::create_signature(class_name.to_string(), &ast_sig);

    SkMethod {
        signature: sig,
        body: SkMethodBody::RustMethodBody{ gen: gen }
    }
}
