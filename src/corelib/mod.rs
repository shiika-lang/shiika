mod array;
mod bool;
mod class;
mod float;
mod fn_x;
mod int;
mod math;
mod never;
mod object;
mod shiika_internal_memory;
mod shiika_internal_ptr;
mod void;
use crate::hir::*;
use crate::library::ImportedItems;
use crate::names::*;
use crate::parser;
use crate::ty;
use std::collections::HashMap;

pub fn create() -> ImportedItems {
    let (sk_classes, sk_methods) = make_classes(rust_body_items());
    ImportedItems {
        sk_classes,
        sk_methods,
        constants: Default::default(),
    }
}

type ClassItem = (
    String,
    Option<ClassFullname>, // superclass
    Vec<SkMethod>,
    Vec<SkMethod>,
    HashMap<String, SkIVar>,
    Vec<String>,
);

fn rust_body_items() -> Vec<ClassItem> {
    let mut ret = vec![
        // Classes
        (
            // `Class` must be created before loading builtin/* because
            // `Meta::XX` inherits `Class`.
            "Class".to_string(),
            Some(class_fullname("Object")),
            Default::default(),
            vec![],
            class::ivars(),
            vec![],
        ),
        (
            "Array".to_string(),
            Some(class_fullname("Object")),
            array::create_methods(),
            vec![],
            HashMap::new(),
            vec!["T".to_string()],
        ),
        (
            "Bool".to_string(),
            Some(class_fullname("Object")),
            bool::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Float".to_string(),
            Some(class_fullname("Object")),
            float::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Int".to_string(),
            Some(class_fullname("Object")),
            int::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Object".to_string(),
            None,
            object::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Void".to_string(),
            Some(class_fullname("Object")),
            void::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Never".to_string(),
            Some(class_fullname("Object")),
            never::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal::Ptr".to_string(),
            Some(class_fullname("Object")),
            shiika_internal_ptr::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        // Modules
        (
            "Math".to_string(),
            Some(class_fullname("Object")),
            vec![],
            math::create_class_methods(),
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika".to_string(),
            Some(class_fullname("Object")),
            vec![],
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal".to_string(),
            Some(class_fullname("Object")),
            vec![],
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal::Memory".to_string(),
            Some(class_fullname("Object")),
            vec![],
            shiika_internal_memory::create_class_methods(),
            HashMap::new(),
            vec![],
        ),
    ];
    ret.append(&mut fn_x::fn_items());
    ret
}

fn make_classes(
    items: Vec<ClassItem>,
) -> (
    HashMap<ClassFullname, SkClass>,
    HashMap<ClassFullname, Vec<SkMethod>>,
) {
    let mut sk_classes = HashMap::new();
    let mut sk_methods = HashMap::new();
    for (name, super_name, imethods, cmethods, ivars, typarams) in items {
        sk_classes.insert(
            ClassFullname(name.to_string()),
            SkClass {
                fullname: class_fullname(&name),
                typarams: typarams
                    .iter()
                    .map(|s| ty::TyParam { name: s.clone() })
                    .collect(),
                superclass_fullname: super_name,
                instance_ty: ty::raw(&name),
                ivars,
                method_sigs: imethods
                    .iter()
                    .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                    .collect(),
                const_is_obj: (name == "Void"),
            },
        );
        sk_methods.insert(class_fullname(&name), imethods);

        if name == "Class" {
            // The class of `Class` is `Class` itself. So we don't need to create again
        } else {
            let meta_ivars = class::ivars(); // `Meta::XX` inherits `Class`
            sk_classes.insert(
                metaclass_fullname(&name),
                SkClass {
                    fullname: metaclass_fullname(&name),
                    typarams: typarams
                        .into_iter()
                        .map(|s| ty::TyParam { name: s })
                        .collect(),
                    superclass_fullname: Some(class_fullname("Class")),
                    instance_ty: ty::meta(&name),
                    ivars: meta_ivars,
                    method_sigs: cmethods
                        .iter()
                        .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                        .collect(),
                    const_is_obj: false,
                },
            );
            sk_methods.insert(metaclass_fullname(&name), cmethods);
        }
    }
    (sk_classes, sk_methods)
}

fn create_method(class_name: &str, sig_str: &str, gen: GenMethodBody) -> SkMethod {
    create_method_generic(class_name, sig_str, gen, &[])
}

fn create_method_generic(
    class_name: &str,
    sig_str: &str,
    gen: GenMethodBody,
    typaram_names: &[String],
) -> SkMethod {
    let mut parser = parser::Parser::new_with_state(sig_str, parser::lexer::LexerState::MethodName);
    let (ast_sig, _) = parser.parse_method_signature().unwrap();
    parser.expect_eof().unwrap();
    let sig = crate::hir::signature::create_signature(
        &class_fullname(class_name),
        &ast_sig,
        typaram_names,
    );

    SkMethod {
        signature: sig,
        body: SkMethodBody::RustMethodBody { gen },
        lvars: vec![],
    }
}
