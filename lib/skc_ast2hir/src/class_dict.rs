use std::collections::HashMap;
mod build_wtable;
mod found_method;
mod indexing;
mod query;
mod type_index;
use anyhow::Result;
pub use found_method::FoundMethod;
use shiika_ast;
use shiika_core::names::*;
use skc_hir::*;

#[derive(Debug, PartialEq)]
pub struct ClassDict<'hir_maker> {
    /// List of classes (without method) collected prior to sk_types
    type_index: type_index::TypeIndex,
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_types: SkTypes,
    /// Imported classes
    imported_classes: &'hir_maker SkTypes,
}

pub fn create<'hir_maker>(
    ast: &shiika_ast::Program,
    // Corelib classes (REFACTOR: corelib should provide methods only)
    initial_sk_types: SkTypes,
    imported_classes: &'hir_maker SkTypes,
) -> Result<ClassDict<'hir_maker>> {
    let defs = ast
        .toplevel_items
        .iter()
        .filter_map(|item| match item {
            shiika_ast::TopLevelItem::Def(x) => Some(x),
            shiika_ast::TopLevelItem::Expr(_) => None,
        })
        .collect::<Vec<_>>();
    let mut dict = ClassDict {
        type_index: type_index::create(&defs, &initial_sk_types, imported_classes),
        sk_types: initial_sk_types,
        imported_classes,
    };
    dict.index_program(&defs)?;
    Ok(dict)
}

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Define ivars of a class
    pub fn define_ivars(&mut self, classname: &ClassFullname, own_ivars: HashMap<String, SkIVar>) {
        let ivars = self.superclass_ivars(classname).unwrap_or_default();
        let class = self.get_class_mut(classname);
        if !class.ivars.is_empty() {
            // The ivars are defined in skc_corelib. Just check that
            // all the ivars are included
            for (k, v) in ivars.iter().chain(own_ivars.iter()) {
                debug_assert!(class.ivars.get(k).unwrap() == v);
            }
            return;
        }
        class.ivars = ivars;
        own_ivars.into_iter().for_each(|(k, v)| {
            class.ivars.insert(k, v);
        });
    }
}

/// Returns signature of `#initialize` inherited from generic class
/// eg.
///   class Foo<A>
///     def initialize(a: A) ...
///   class Bar<S, T> : Foo<Array<T>>
///     # no explicit initialize
/// Foo will have `#initialize(a: Array<T>)`
fn specialized_initialize(sig: &MethodSignature, superclass: &Superclass) -> MethodSignature {
    sig.specialize(superclass.ty().tyargs(), &[])
}
