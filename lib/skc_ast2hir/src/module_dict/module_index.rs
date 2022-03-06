use crate::parse_typarams;
use shiika_ast;
use shiika_core::{names::*, ty};
use skc_hir::*;
use std::collections::HashMap;

/// Set of pair of class name and its typaram names
pub type ModuleIndex = HashMap<ModuleFullname, Vec<ty::TyParam>>;

/// Collect class names in the program
pub fn create(
    toplevel_defs: &[&shiika_ast::Definition],
    initial_sk_classes: &SkModules,
    imported_classes: &SkModules,
) -> ModuleIndex {
    let mut cindex = HashMap::new();
    index_sk_classes(&mut cindex, initial_sk_classes);
    index_sk_classes(&mut cindex, imported_classes);
    index_toplevel_defs(&mut cindex, toplevel_defs);
    cindex
}

fn index_sk_classes(cindex: &mut ModuleIndex, sk_classes: &SkModules) {
    for (name, class) in sk_classes {
        cindex.insert(name.clone(), class.typarams.clone());
    }
}

fn index_toplevel_defs(cindex: &mut ModuleIndex, toplevel_defs: &[&shiika_ast::Definition]) {
    let namespace = Namespace::root();
    for def in toplevel_defs {
        match def {
            shiika_ast::Definition::ClassDefinition {
                name,
                typarams,
                defs,
                ..
            } => index_class(cindex, &namespace, name, parse_typarams(typarams), defs),
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
    cindex: &mut ModuleIndex,
    namespace: &Namespace,
    firstname: &ModuleFirstname,
    typarams: Vec<ty::TyParam>,
    defs: &[shiika_ast::Definition],
) {
    let fullname = namespace.module_fullname(firstname);
    cindex.insert(fullname, typarams);
    let inner_namespace = namespace.add(firstname);
    for def in defs {
        match def {
            shiika_ast::Definition::ClassDefinition {
                name,
                typarams,
                defs,
                ..
            } => {
                index_class(
                    cindex,
                    &inner_namespace,
                    name,
                    parse_typarams(typarams),
                    defs,
                );
            }
            shiika_ast::Definition::EnumDefinition {
                name,
                typarams,
                cases,
                ..
            } => {
                index_enum(
                    cindex,
                    &inner_namespace,
                    name,
                    parse_typarams(typarams),
                    cases,
                );
            }
            _ => (),
        }
    }
}

fn index_enum(
    cindex: &mut ModuleIndex,
    namespace: &Namespace,
    firstname: &ModuleFirstname,
    typarams: Vec<ty::TyParam>,
    cases: &[shiika_ast::EnumCase],
) {
    let fullname = namespace.module_fullname(firstname);
    cindex.insert(fullname, typarams.to_vec());

    let inner_namespace = namespace.add(firstname);
    for case in cases {
        let case_fullname = inner_namespace.module_fullname(&case.name);
        cindex.insert(case_fullname, typarams.to_vec());
    }
}
