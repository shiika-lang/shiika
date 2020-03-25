mod bool;
mod float;
mod int;
mod math;
mod object;
mod void;
mod never;
mod string;
use std::collections::HashMap;
use crate::names::*;
use crate::ty;
use crate::hir::*;

pub struct Stdlib {
    pub sk_classes: HashMap<ClassFullname, SkClass>,
    pub sk_methods: HashMap<ClassFullname, Vec<SkMethod>>,
}

impl Stdlib {
    /// Create empty Stdlib (for tests)
    pub fn empty() -> Stdlib {
        Stdlib { 
            sk_classes: HashMap::new(),
            sk_methods: HashMap::new(),
        }
    }

    pub fn create() -> Stdlib {
        let mut sk_classes = HashMap::new();
        let mut sk_methods = HashMap::new();
        let items = vec![
            ("Bool", bool::create_methods(), vec![]),
            ("Float", float::create_methods(), vec![]),
            ("Int", int::create_methods(), vec![]),
            ("Object", object::create_methods(), vec![]),
            ("Void", void::create_methods(), vec![]),
            ("Never", never::create_methods(), vec![]),
            ("Math", vec![], math::create_class_methods()),
            ("String", string::create_methods(), vec![]),
        ];
        for t in items.into_iter() {
            let (name, imethods, cmethods) = t;
            let super_name = if name == "Object" { None }
                             else { Some(ClassFullname("Object".to_string())) };
            sk_classes.insert(
                ClassFullname(name.to_string()),
                SkClass {
                    fullname: ClassFullname(name.to_string()),
                    superclass_fullname: super_name,
                    instance_ty: ty::raw(name),
                    ivars: HashMap::new(),
                    method_sigs: imethods.iter().map(|x|
                        (x.signature.first_name().clone(), x.signature.clone())
                    ).collect(),
                }
            );
            sk_classes.insert(
                ClassFullname("Meta:".to_string() + name),
                SkClass {
                    fullname: ClassFullname("Meta:".to_string() + name),
                    superclass_fullname: Some(ClassFullname("Meta:Object".to_string())),
                    instance_ty: ty::meta(name),
                    ivars: HashMap::new(),
                    method_sigs: cmethods.iter().map(|x|
                        (x.signature.first_name().clone(), x.signature.clone())
                    ).collect(),
                }
            );
            sk_methods.insert(
                ClassFullname(name.to_string()),
                imethods.into_iter().chain(cmethods).collect()
            );
        };
        Stdlib { sk_classes, sk_methods }
    }
}

fn create_method(class_name: &str,
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
