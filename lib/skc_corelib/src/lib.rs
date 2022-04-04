pub mod class;
mod fn_x;
pub mod rustlib_methods;
use shiika_core::names::*;
use shiika_core::ty::{self, Erasure};
use skc_hir::*;
use std::collections::HashMap;

pub struct Corelib {
    pub sk_types: SkTypes,
    pub sk_methods: SkMethods,
}

/// Create a `Corelib`
pub fn create() -> Corelib {
    let (sk_types, sk_methods) = make_classes(rust_body_items());

    Corelib {
        sk_types,
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
            "Metaclass".to_string(),
            Some(Superclass::simple("Class")),
            Default::default(),
            vec![],
            class::ivars(),
            vec![],
        ),
        (
            "String".to_string(),
            Some(Superclass::simple("Object")),
            Default::default(),
            vec![],
            Default::default(),
            vec![],
        ),
        (
            "Array".to_string(),
            Some(Superclass::simple("Object")),
            vec![],
            vec![],
            HashMap::new(),
            vec!["T".to_string()],
        ),
        (
            "Bool".to_string(),
            Some(Superclass::simple("Object")),
            vec![],
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Float".to_string(),
            Some(Superclass::simple("Object")),
            vec![], //float::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Int".to_string(),
            Some(Superclass::simple("Object")),
            vec![], //int::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Object".to_string(),
            None,
            vec![object_initialize()], // object::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Void".to_string(),
            Some(Superclass::simple("Object")),
            vec![],
            vec![],
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal::Ptr".to_string(),
            Some(Superclass::simple("Object")),
            vec![], //shiika_internal_ptr::create_methods(),
            vec![],
            HashMap::new(),
            vec![],
        ),
        // Modules
        (
            "Math".to_string(),
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
            vec![], //shiika_internal_memory::create_class_methods(),
            HashMap::new(),
            vec![],
        ),
    ];
    ret.append(&mut fn_x::fn_items());
    ret
}

#[allow(clippy::if_same_then_else)]
fn make_classes(
    items: Vec<ClassItem>,
) -> (SkTypes, SkMethods) {
    let mut sk_types = HashMap::new();
    let mut sk_methods = HashMap::new();
    for (name, superclass, imethods, cmethods, ivars, typarams) in items {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta(&name),
            typarams: typarams.iter().map(ty::TyParam::new).collect(),
            method_sigs: imethods
                .iter()
                .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                .collect(),
            foreign: false,
        };
        let sk_class = SkClass::nonmeta(base, superclass)
            .ivars(ivars)
            .const_is_obj(name == "Void");
        sk_types.insert(
            ClassFullname(name.to_string()),
            sk_class.into()
        );
        sk_methods.insert(class_fullname(&name), imethods);

        if name == "Metaclass" {
            // The class of `Metaclass` is `Metaclass` itself. So we don't need to create again
        } else {
            let base = SkTypeBase {
                erasure: Erasure::meta(&name),
                typarams: typarams.into_iter().map(ty::TyParam::new).collect(),
                method_sigs: cmethods
                    .iter()
                    .map(|x| (x.signature.first_name().clone(), x.signature.clone()))
                    .collect(),
                foreign: false,
            };
            let sk_class = SkClass::meta(base)
                .ivars(class::ivars());
            sk_types.insert(
                metaclass_fullname(&name),
                sk_class.into()
            );
            sk_methods.insert(metaclass_fullname(&name), cmethods);
        }
    }
    (sk_types, sk_methods)
}

fn _convert_typ(
    typ: &ConstName,
    class_typarams: &[String],
    method_typarams: &[shiika_ast::AstTyParam],
) -> ty::TermTy {
    let s = typ.names.join("::");
    if let Some(idx) = class_typarams.iter().position(|t| s == *t) {
        ty::typaram_ref(s, ty::TyParamKind::Class, idx).into_term_ty()
    } else if let Some(idx) = method_typarams.iter().position(|t| s == t.name) {
        ty::typaram_ref(s, ty::TyParamKind::Method, idx).into_term_ty()
    } else {
        let tyargs = typ
            .args
            .iter()
            .map(|arg| _convert_typ(arg, class_typarams, method_typarams))
            .collect::<Vec<_>>();
        ty::nonmeta(&typ.names, tyargs)
    }
}

fn object_initialize() -> SkMethod {
    let sig = MethodSignature {
        fullname: method_fullname_raw("Object", "initialize"),
        ret_ty: ty::raw("Void"),
        params: vec![],
        typarams: vec![],
    };
    SkMethod {
        signature: sig,
        body: SkMethodBody::Normal {
            exprs: Hir::expressions(vec![Hir::const_ref(
                ty::raw("Void"),
                toplevel_const("Void"),
            )]),
        },
        lvars: vec![],
    }
}
