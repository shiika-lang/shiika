use std::collections::HashMap;
mod build_wtable;
mod found_method;
mod indexing;
mod query;
pub mod type_index;
use anyhow::Result;
pub use found_method::FoundMethod;
use shiika_ast;
use shiika_core::names::*;
use skc_hir::*;
use type_index::TypeIndex;

type RustMethods = HashMap<TypeFullname, Vec<MethodSignature>>;

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
    rust_methods: RustMethods,
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
        rust_methods: Default::default(),
    };
    dict.index_program(&defs)?;
    Ok(dict)
}

pub fn create_for_corelib<'hir_maker>(
    defs: &[&shiika_ast::Definition],
    imported_classes: &'hir_maker SkTypes,
    sk_types: SkTypes,
    type_index: TypeIndex,
    rust_method_sigs: &[MethodSignature],
) -> Result<ClassDict<'hir_maker>> {
    let mut dict = ClassDict {
        type_index,
        sk_types,
        imported_classes,
        rust_methods: index_rust_method_sigs(rust_method_sigs),
    };
    dict.index_program(&defs)?;
    Ok(dict)
}

fn index_rust_method_sigs(rust_method_sigs: &[MethodSignature]) -> RustMethods {
    let mut rust_methods = HashMap::new();
    for sig in rust_method_sigs {
        let typename = sig.fullname.type_name.clone();
        let v: &mut Vec<MethodSignature> = rust_methods.entry(typename).or_default();
        v.push(sig.clone());
    }
    rust_methods
}

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Define ivars of a class
    pub fn define_ivars(&mut self, classname: &ClassFullname, own_ivars: HashMap<String, SkIVar>) {
        let ivars = self.superclass_ivars(classname).unwrap_or_default();
        let class = self.get_class_mut(classname);
        if !classname.is_meta() && !class.ivars.is_empty() {
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
