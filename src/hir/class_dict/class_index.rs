use crate::ast;
use crate::hir::*;
use crate::names::*;
use std::collections::HashMap;

/// Set of pair of class name and its typaram names
pub type ClassIndex = HashMap<ClassFullname, Vec<TyParam>>;

/// Collect class names in the program
pub fn create(
    toplevel_defs: &[&ast::Definition],
    initial_sk_classes: &SkClasses,
    imported_classes: &SkClasses,
) -> ClassIndex {
    let mut cindex = HashMap::new();
    index_sk_classes(&mut cindex, initial_sk_classes);
    index_sk_classes(&mut cindex, imported_classes);
    index_toplevel_defs(&mut cindex, toplevel_defs);
    cindex
}

fn index_sk_classes(cindex: &mut ClassIndex, sk_classes: &SkClasses) {
    for (name, class) in sk_classes {
        cindex.insert(name.clone(), class.typarams.clone());
    }
}

fn index_toplevel_defs(cindex: &mut ClassIndex, toplevel_defs: &[&ast::Definition]) {
    let namespace = Namespace::root();
    for def in toplevel_defs {
        match def {
            ast::Definition::ClassDefinition {
                name,
                typarams,
                defs,
                ..
            } => index_class(cindex, &namespace, name, typarams, defs),
            ast::Definition::EnumDefinition {
                name,
                typarams,
                cases,
                ..
            } => index_enum(cindex, &namespace, name, typarams, cases),
            _ => (),
        }
    }
}

fn index_class(
    cindex: &mut ClassIndex,
    namespace: &Namespace,
    firstname: &ClassFirstname,
    typarams: &[ty::TyParam],
    defs: &[ast::Definition],
) {
    let fullname = namespace.class_fullname(firstname);
    cindex.insert(fullname, typarams.to_vec());
    let inner_namespace = namespace.add(firstname);
    for def in defs {
        match def {
            ast::Definition::ClassDefinition {
                name,
                typarams,
                defs,
                ..
            } => {
                index_class(cindex, &inner_namespace, name, typarams, defs);
            }
            ast::Definition::EnumDefinition {
                name,
                typarams,
                cases,
                ..
            } => {
                index_enum(cindex, &inner_namespace, name, typarams, cases);
            }
            _ => (),
        }
    }
}

fn index_enum(
    cindex: &mut ClassIndex,
    namespace: &Namespace,
    firstname: &ClassFirstname,
    typarams: &[ty::TyParam],
    cases: &[ast::EnumCase],
) {
    let fullname = namespace.class_fullname(firstname);
    cindex.insert(fullname, typarams.to_vec());

    let inner_namespace = namespace.add(firstname);
    for case in cases {
        let case_fullname = inner_namespace.class_fullname(&case.name);
        cindex.insert(case_fullname, typarams.to_vec());
    }
}
