use std::collections::HashMap;
mod build_wtable;
mod found_method;
mod indexing;
mod query;
pub mod type_index;
use anyhow::Result;
pub use found_method::{CallType, FoundMethod};
pub use indexing::RustMethods;
use shiika_ast::{self};
use shiika_core::names::*;
use skc_hir::*;
use type_index::TypeIndex;

#[derive(Debug)]
pub struct ClassDict<'hir_maker> {
    /// List of classes (without method) collected prior to sk_types
    type_index: type_index::TypeIndex,
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_types: SkTypes,
    /// Imported classes (TODO: Rename to `imported_types`)
    pub imported_classes: &'hir_maker SkTypes,
}

pub fn new<'hir_maker>(
    type_index: TypeIndex,
    imported_classes: &'hir_maker SkTypes,
) -> ClassDict<'hir_maker> {
    ClassDict {
        type_index,
        sk_types: Default::default(),
        imported_classes,
    }
}

pub fn create<'hir_maker>(
    defs: &[&shiika_ast::Definition],
    type_index: TypeIndex,
    imported_classes: &'hir_maker SkTypes,
) -> Result<ClassDict<'hir_maker>> {
    let mut dict = ClassDict {
        type_index,
        sk_types: Default::default(),
        imported_classes,
    };
    dict.index_program(defs, HashMap::new())?;
    Ok(dict)
}

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Define ivars of a class
    pub fn define_ivars(&mut self, classname: &ClassFullname, own_ivars: HashMap<String, SkIVar>) {
        let superclass = &self.get_class(classname).superclass.clone();
        let mut ivars = self.superclass_ivars(superclass).unwrap_or_default();
        let class = self.get_class_mut(classname);
        // Preserve ivars set by bootstrap that aren't from the superclass
        for (k, v) in &class.ivars {
            ivars.entry(k.clone()).or_insert(v.clone());
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
fn specialized_initialize(sig: &MethodSignature, superclass: &Supertype) -> MethodSignature {
    sig.specialize(superclass.type_args(), &[])
}
