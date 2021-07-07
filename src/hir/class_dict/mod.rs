mod class_index;
mod indexing;
mod query;
use crate::ast;
use crate::error::*;
use crate::hir::*;
use crate::names::*;

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
    ast: &ast::Program,
    // Corelib classes (REFACTOR: corelib should provide methods only)
    initial_sk_classes: SkClasses,
    imported_classes: &'hir_maker SkClasses,
) -> Result<ClassDict<'hir_maker>, Error> {
    let defs = ast
        .toplevel_items
        .iter()
        .filter_map(|item| match item {
            ast::TopLevelItem::Def(x) => Some(x),
            ast::TopLevelItem::Expr(_) => None,
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
    /// Returns information for creating class constants
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
    pub fn define_ivars(&mut self, classname: &ClassFullname, own_ivars: HashMap<String, SkIVar>) {
        let ivars = self
            .superclass_ivars(classname)
            .unwrap_or_else(|| Default::default());
        let class = self.get_class_mut(&classname);
        debug_assert!(class.ivars.is_empty());
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
