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
    initial_sk_types: &SkTypes,
    imported_classes: &SkTypes,
) -> TypeIndex {
    let mut cindex = HashMap::new();
    index_sk_types(&mut cindex, initial_sk_types);
    index_sk_types(&mut cindex, imported_classes);
    index_defs(&mut cindex, &Namespace::root(), toplevel_defs);
    cindex
}

fn index_sk_types(cindex: &mut TypeIndex, sk_types: &SkTypes) {
    for (name, class) in sk_types {
        cindex.insert(name.clone().into(), class.base().typarams.clone());
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
            } => index_class(cindex, &namespace, name, parse_typarams(typarams), defs),
            shiika_ast::Definition::ModuleDefinition {
                name,
                typarams,
                defs,
                ..
            } => index_module(cindex, &namespace, name, parse_typarams(typarams), defs),
            shiika_ast::Definition::EnumDefinition {
                name,
                typarams,
                cases,
                ..
            } => index_enum(cindex, &namespace, name, parse_typarams(typarams), cases),
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
    let fullname = namespace.class_fullname(firstname);
    cindex.insert(fullname.into(), typarams);
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
    let fullname = namespace.module_fullname(firstname);
    cindex.insert(fullname.into(), typarams);
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
    let fullname = namespace.class_fullname(firstname);
    cindex.insert(fullname.into(), typarams.to_vec());

    let inner_namespace = namespace.add(firstname.0.clone());
    for case in cases {
        let case_fullname = inner_namespace.class_fullname(&case.name);
        cindex.insert(case_fullname.into(), typarams.to_vec());
    }
}
