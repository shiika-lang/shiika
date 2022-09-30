use crate::parse_typarams;
use shiika_ast;
use shiika_core::{names::*, ty};
use skc_hir::*;
use std::collections::HashMap;

/// Set of pair of class name and its typaram names
pub type TypeIndex = HashMap<TypeFullname, Vec<ty::TyParam>>;

/// Collect class names in the program
pub fn create(
    toplevel_defs: &[&shiika_ast::Definition],
    corelib_sk_types: &SkTypes,
    imported_classes: &SkTypes,
) -> TypeIndex {
    let mut cindex = HashMap::new();
    index_sk_types(&mut cindex, corelib_sk_types, true);
    index_sk_types(&mut cindex, imported_classes, false);
    index_defs(&mut cindex, &Namespace::root(), toplevel_defs);
    cindex
}

fn index_sk_types(cindex: &mut TypeIndex, sk_types: &SkTypes, create_meta: bool) {
    for (name, class) in &sk_types.0 {
        cindex.insert(name.clone(), class.base().typarams.clone());
        if create_meta {
            let meta_name = name.meta_name();
            cindex.insert(meta_name.into(), Default::default());
        }
    }
}

fn index_defs(cindex: &mut TypeIndex, namespace: &Namespace, defs: &[&shiika_ast::Definition]) {
    for def in defs {
        match def {
            shiika_ast::Definition::ClassDefinition {
                name,
                typarams,
                defs,
                ..
            } => index_class(cindex, namespace, name, parse_typarams(typarams), defs),
            shiika_ast::Definition::ModuleDefinition {
                name,
                typarams,
                defs,
                ..
            } => index_module(cindex, namespace, name, parse_typarams(typarams), defs),
            shiika_ast::Definition::EnumDefinition {
                name,
                typarams,
                cases,
                ..
            } => index_enum(cindex, namespace, name, parse_typarams(typarams), cases),
            _ => (),
        }
    }
}

fn index_class(
    cindex: &mut TypeIndex,
    namespace: &Namespace,
    firstname: &ClassFirstname,
    typarams: Vec<ty::TyParam>,
    defs: &[shiika_ast::Definition],
) {
    let fullname = namespace.type_fullname(&firstname.0);
    insert_class_and_metaclass(cindex, fullname, typarams);
    let inner_namespace = namespace.add(firstname.0.clone());
    index_defs(cindex, &inner_namespace, &defs.iter().collect::<Vec<_>>());
}

fn index_module(
    cindex: &mut TypeIndex,
    namespace: &Namespace,
    firstname: &ModuleFirstname,
    typarams: Vec<ty::TyParam>,
    defs: &[shiika_ast::Definition],
) {
    let fullname = namespace.type_fullname(&firstname.0);
    insert_class_and_metaclass(cindex, fullname, typarams);
    let inner_namespace = namespace.add(firstname.0.clone());
    index_defs(cindex, &inner_namespace, &defs.iter().collect::<Vec<_>>());
}

fn index_enum(
    cindex: &mut TypeIndex,
    namespace: &Namespace,
    firstname: &ClassFirstname,
    typarams: Vec<ty::TyParam>,
    cases: &[shiika_ast::EnumCase],
) {
    let fullname = namespace.type_fullname(&firstname.0);
    insert_class_and_metaclass(cindex, fullname, typarams.clone());
    let inner_namespace = namespace.add(firstname.0.clone());
    for case in cases {
        let case_fullname = inner_namespace.type_fullname(&case.name.0);
        insert_class_and_metaclass(cindex, case_fullname, typarams.clone());
    }
}

fn insert_class_and_metaclass(
    cindex: &mut TypeIndex,
    name: TypeFullname,
    typarams: Vec<ty::TyParam>,
) {
    let meta_name = name.meta_name();
    cindex.insert(name, typarams);
    cindex.insert(meta_name.into(), Default::default());
}
