mod bool;
mod float;
mod int;
mod math;
mod never;
mod object;
mod shiika_internal_memory;
mod shiika_internal_ptr;
mod string;
mod void;
use crate::hir::*;
use crate::names::*;
use crate::parser;
use crate::ty;
use std::collections::HashMap;

pub struct Corelib {
    pub sk_classes: HashMap<ClassFullname, SkClass>,
    pub sk_methods: HashMap<ClassFullname, Vec<SkMethod>>,
}

impl Corelib {
    /// Create empty Corelib (for tests)
    pub fn empty() -> Corelib {
        Corelib {
            sk_classes: HashMap::new(),
            sk_methods: HashMap::new(),
        }
    }

    pub fn create() -> Corelib {
        let items = rust_body_items();
        let (sk_classes, sk_methods) = make_classes(items);
        Corelib {
            sk_classes,
            sk_methods,
        }
    }
}

type ClassItem = (
    &'static str,
    Vec<SkMethod>,
    Vec<SkMethod>,
    HashMap<String, SkIVar>,
);

fn rust_body_items() -> Vec<ClassItem> {
    vec![
        // Classes
        ("Bool", bool::create_methods(), vec![], HashMap::new()),
        ("Float", float::create_methods(), vec![], HashMap::new()),
        ("Int", int::create_methods(), vec![], HashMap::new()),
        ("Object", object::create_methods(), vec![], HashMap::new()),
        ("Void", void::create_methods(), vec![], HashMap::new()),
        ("Never", never::create_methods(), vec![], HashMap::new()),
        ("String", string::create_methods(), vec![], string::ivars()),
        ("Class", vec![], vec![], HashMap::new()),
        (
            "Shiika::Internal::Ptr",
            shiika_internal_ptr::create_methods(),
            vec![],
            HashMap::new(),
        ),
        // Modules
        ("Math", vec![], math::create_class_methods(), HashMap::new()),
        ("Shiika", vec![], vec![], HashMap::new()),
        ("Shiika::Internal", vec![], vec![], HashMap::new()),
        (
            "Shiika::Internal::Memory",
            vec![],
            shiika_internal_memory::create_class_methods(),
            HashMap::new(),
        ),
    ]
}

fn make_classes(
    items: Vec<ClassItem>,
) -> (
    HashMap<ClassFullname, SkClass>,
    HashMap<ClassFullname, Vec<SkMethod>>,
) {
    let mut sk_classes = HashMap::new();
    let mut sk_methods = HashMap::new();
    for (name, imethods, cmethods, ivars) in items {
        let super_name = if name == "Object" {
            None
        } else {
            Some(ClassFullname("Object".to_string()))
        };
        sk_classes.insert(
            ClassFullname(name.to_string()),
            SkClass {
                fullname: class_fullname(name),
                typarams: vec![],
                superclass_fullname: super_name,
                instance_ty: ty::raw(name),
                ivars,
                method_sigs: imethods
                    .iter()
                    .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                    .collect(),
            },
        );

        let mut meta_ivars = HashMap::new();
        meta_ivars.insert(
            "@name".to_string(),
            SkIVar {
                name: "@name".to_string(),
                idx: 0,
                ty: ty::raw("String"),
                readonly: true,
            },
        );
        sk_classes.insert(
            metaclass_fullname(name),
            SkClass {
                fullname: metaclass_fullname(name),
                typarams: vec![],
                superclass_fullname: Some(class_fullname("Class")),
                instance_ty: ty::meta(name),
                ivars: meta_ivars,
                method_sigs: cmethods
                    .iter()
                    .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                    .collect(),
            },
        );

        sk_methods.insert(class_fullname(name), imethods);
        sk_methods.insert(metaclass_fullname(name), cmethods);
    }
    (sk_classes, sk_methods)
}

fn create_method(class_name: &str, sig_str: &str, gen: GenMethodBody) -> SkMethod {
    let mut parser = parser::Parser::new_with_state(sig_str, parser::lexer::LexerState::MethodName);
    let (ast_sig, _) = parser.parse_method_signature().unwrap();
    parser.expect_eof().unwrap();
    let sig =
        crate::hir::signature::create_signature(&class_fullname(class_name), &ast_sig, &[]);

    SkMethod {
        signature: sig,
        body: SkMethodBody::RustMethodBody { gen },
    }
}
