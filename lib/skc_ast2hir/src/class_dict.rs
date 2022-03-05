use std::collections::HashMap;
mod class_index;
mod indexing;
mod query;
use anyhow::Result;
use shiika_ast;
use shiika_core::names::*;
use skc_hir::*;

#[derive(Debug, PartialEq)]
pub struct ClassDict<'hir_maker> {
    /// List of classes (without method) collected prior to sk_classes
    class_index: class_index::ClassIndex,
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_classes: SkClasses,
    /// Imported classes
    imported_classes: &'hir_maker SkClasses,
}

pub fn create<'hir_maker>(
    ast: &shiika_ast::Program,
    // Corelib classes (REFACTOR: corelib should provide methods only)
    initial_sk_classes: SkClasses,
    imported_classes: &'hir_maker SkClasses,
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
        class_index: class_index::create(&defs, &initial_sk_classes, imported_classes),
        sk_classes: initial_sk_classes,
        imported_classes,
    };
    dict.index_program(&defs)?;
    Ok(dict)
}

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Returns information for creating class constants i.e. a list of
    /// `(name, const_is_obj)`
    pub fn constant_list(&self) -> Vec<(String, bool)> {
        self.sk_classes
            .iter()
            .filter_map(|(name, class)| {
                if name.is_meta() {
                    None
                } else {
                    Some((name.0.clone(), class.const_is_obj))
                }
            })
            .collect()
    }

    /// Define ivars of a class
    pub fn define_ivars(&mut self, classname: &ModuleFullname, own_ivars: HashMap<String, SkIVar>) {
        let ivars = self
            .superclass_ivars(classname)
            .unwrap_or_else(Default::default);
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
