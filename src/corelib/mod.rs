mod array;
mod bool;
mod class;
mod float;
mod fn_x;
mod int;
mod math;
mod object;
mod shiika_internal_memory;
mod shiika_internal_ptr;
mod void;
use crate::hir::*;
use crate::names::*;
use crate::parser;
use crate::ty;
use std::collections::HashMap;

pub struct Corelib {
    pub sk_classes: SkClasses,
    pub sk_methods: SkMethods,
}

pub fn create() -> Corelib {
    let (sk_classes, sk_methods) = make_classes(rust_body_items());
    Corelib {
        sk_classes,
        sk_methods,
    }
}

type ClassItem = (
    String,
    Option<Superclass>,
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
            Some(Superclass::simple("Object")),
            Default::default(),
            vec![],
            class::ivars(),
            vec![],
        ),
        (
            "Array".to_string(),
            Some(Superclass::simple("Object")),
            array::create_methods(),
            vec![],
            HashMap::new(),
            vec!["T".to_string()],
        ),
        (
            "Bool".to_string(),
            Some(Superclass::simple("Object")),
            bool::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Float".to_string(),
            Some(Superclass::simple("Object")),
            float::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Int".to_string(),
            Some(Superclass::simple("Object")),
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
            Some(Superclass::simple("Object")),
            void::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal::Ptr".to_string(),
            Some(Superclass::simple("Object")),
            shiika_internal_ptr::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        // Modules
        (
            "Math".to_string(),
            Some(Superclass::simple("Object")),
            vec![],
            math::create_class_methods(),
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika".to_string(),
            Some(Superclass::simple("Object")),
            vec![],
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal".to_string(),
            Some(Superclass::simple("Object")),
            vec![],
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal::Memory".to_string(),
            Some(Superclass::simple("Object")),
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
    for (name, superclass, imethods, cmethods, ivars, typarams) in items {
        sk_classes.insert(
            ClassFullname(name.to_string()),
            SkClass {
                fullname: class_fullname(&name),
                typarams: typarams
                    .iter()
                    .map(|s| ty::TyParam { name: s.clone() })
                    .collect(),
                superclass,
                instance_ty: ty::raw(&name),
                ivars,
                method_sigs: imethods
                    .iter()
                    .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                    .collect(),
                const_is_obj: (name == "Void"),
                foreign: false,
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
                    superclass: Some(Superclass::simple("Class")),
                    instance_ty: ty::meta(&name),
                    ivars: meta_ivars,
                    method_sigs: cmethods
                        .iter()
                        .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                        .collect(),
                    const_is_obj: false,
                    foreign: false,
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

    let ret_ty = if let Some(typ) = &ast_sig.ret_typ {
        _convert_typ(typ, typaram_names, &ast_sig.typarams)
    } else {
        ty::raw("Void")
    };
    let params = ast_sig
        .params
        .iter()
        .map(|param| MethodParam {
            name: param.name.to_string(),
            ty: _convert_typ(&param.typ, typaram_names, &ast_sig.typarams),
        })
        .collect();
    let sig = MethodSignature {
        fullname: method_fullname(&class_fullname(class_name), &ast_sig.name.0),
        ret_ty,
        params,
        typarams: ast_sig.typarams.clone(),
    };
    SkMethod {
        signature: sig,
        body: SkMethodBody::RustMethodBody { gen },
        lvars: vec![],
    }
}

fn _convert_typ(
    typ: &ConstName,
    class_typarams: &[String],
    method_typarams: &[String],
) -> ty::TermTy {
    let s = typ.names.join("::");
    if let Some(idx) = class_typarams.iter().position(|t| s == *t) {
        ty::typaram(s, ty::TyParamKind::Class, idx)
    } else if let Some(idx) = method_typarams.iter().position(|t| s == *t) {
        ty::typaram(s, ty::TyParamKind::Method, idx)
    } else {
        let tyargs = typ
            .args
            .iter()
            .map(|arg| _convert_typ(arg, class_typarams, method_typarams))
            .collect::<Vec<_>>();
        ty::nonmeta(&typ.names, tyargs)
    }
}
