pub mod class;
mod fn_x;
pub mod rustlib_methods;
use shiika_core::names::*;
use shiika_core::ty::{self, Erasure};
use skc_hir::*;
use std::collections::HashMap;

pub struct Corelib {
    pub sk_types: SkTypes,
}

/// Create a `Corelib`
pub fn create() -> Corelib {
    let sk_types = make_classes(rust_body_items());
    Corelib { sk_types }
}

type ClassItem = (
    String,
    Option<Supertype>,
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
            Some(Supertype::simple("Object")),
            class::ivars(),
            vec![],
        ),
        (
            "Metaclass".to_string(),
            Some(Supertype::simple("Class")),
            class::ivars(),
            vec![],
        ),
        (
            "String".to_string(),
            Some(Supertype::simple("Object")),
            Default::default(),
            vec![],
        ),
        (
            "Array".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec!["T".to_string()],
        ),
        (
            "Bool".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec![],
        ),
        (
            "Float".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec![],
        ),
        (
            "Int".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec![],
        ),
        ("Object".to_string(), None, HashMap::new(), vec![]),
        (
            "Void".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal::Ptr".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec![],
        ),
        // Modules
        (
            "Math".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec![],
        ),
        (
            "Shiika::Internal::Memory".to_string(),
            Some(Supertype::simple("Object")),
            HashMap::new(),
            vec![],
        ),
    ];
    ret.append(&mut fn_x::fn_items());
    ret
}

#[allow(clippy::if_same_then_else)]
fn make_classes(items: Vec<ClassItem>) -> SkTypes {
    let mut sk_types = HashMap::new();
    for (name, superclass, ivars, typarams) in items {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta(&name),
            typarams: typarams.iter().map(ty::TyParam::new).collect(),
            method_sigs: Default::default(),
            foreign: false,
        };
        let sk_class = SkClass::nonmeta(base, superclass)
            .ivars(ivars)
            .const_is_obj(name == "Void");
        sk_types.insert(ClassFullname(name.to_string()).into(), sk_class.into());

        if name == "Metaclass" {
            // The class of `Metaclass` is `Metaclass` itself. So we don't need to create again
        } else {
            let base = SkTypeBase {
                erasure: Erasure::meta(&name),
                typarams: typarams.into_iter().map(ty::TyParam::new).collect(),
                method_sigs: Default::default(),
                foreign: false,
            };
            let sk_class = SkClass::meta(base).ivars(class::ivars());
            sk_types.insert(metaclass_fullname(&name).into(), sk_class.into());
        }
    }
    SkTypes::new(sk_types)
}
