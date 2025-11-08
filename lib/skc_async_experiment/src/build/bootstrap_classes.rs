use shiika_core::ty::{self, Erasure};
use skc_ast2hir::class_dict::ClassDict;
use skc_hir::{MethodSignatures, SkTypeBase, Supertype};
use std::collections::HashMap;

/// Register basic classes to ClassDict to bootstrap because some
/// core classes are mutually dependent (eg. Class <-> Object)
pub fn add_to(class_dict: &mut ClassDict) {
    let class_ivars = HashMap::from([(
        "name".to_string(),
        skc_hir::SkIVar {
            name: "name".to_string(),
            ty: ty::raw("String"),
            idx: 0,
            readonly: true,
        },
    )]);

    // Add `Object`
    class_dict.add_type(
        skc_hir::SkClass::nonmeta(
            SkTypeBase {
                erasure: Erasure::nonmeta("Object"),
                typarams: Default::default(),
                method_sigs: MethodSignatures::new(),
                foreign: false,
            },
            None,
        )
        .inheritable(true),
    );
    class_dict.add_type(
        skc_hir::SkClass::meta(SkTypeBase {
            erasure: Erasure::meta("Object"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::new(),
            foreign: false,
        })
        .ivars(class_ivars.clone()),
    );

    // Add `Class`
    class_dict.add_type(
        skc_hir::SkClass::nonmeta(
            SkTypeBase {
                erasure: Erasure::nonmeta("Class"),
                typarams: Default::default(),
                method_sigs: MethodSignatures::new(),
                foreign: false,
            },
            Some(Supertype::simple("Object")),
        )
        .inheritable(true)
        .ivars(class_ivars.clone()),
    );
    class_dict.add_type(
        skc_hir::SkClass::meta(SkTypeBase {
            erasure: Erasure::meta("Class"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::new(),
            foreign: false,
        })
        .ivars(class_ivars.clone()),
    );
}
