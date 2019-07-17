mod float;
mod int;
mod math;
mod object;
mod void;
use crate::names::*;
use crate::ty;
use crate::hir::*;

pub fn create_classes() -> Vec<SkClass> {
    let mut v = vec![];
    let items = vec![
        ("Float", float::create_methods(), vec![]),
        ("Int", int::create_methods(), vec![]),
        ("Object", object::create_methods(), vec![]),
        ("Void", void::create_methods(), vec![]),
        ("Math", vec![], math::create_class_methods()),
    ];
    for t in items.into_iter() {
        let (name, imethods, cmethods) = t;
        v.append(&mut vec![
            SkClass {
                fullname: ClassFullname(name.to_string()),
                instance_ty: ty::raw(name),
                methods: imethods,
            },
            SkClass {
                fullname: ClassFullname("Meta:".to_string() + name),
                instance_ty: ty::meta(name),
                methods: cmethods,
            },
        ]);
    };
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
