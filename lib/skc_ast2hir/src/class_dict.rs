use std::collections::HashMap;
mod build_wtable;
mod found_method;
mod indexing;
mod query;
pub mod type_index;
use anyhow::Result;
pub use found_method::{CallType, FoundMethod};
use shiika_ast::{self};
use shiika_core::names::*;
use skc_hir::*;
use type_index::TypeIndex;

#[derive(Debug)]
pub struct ClassDict<'hir_maker> {
    /// Type annotations to check after indexing, while retaining source locations.
    /// Disabled once indexing finishes; expression annotations are checked separately.
    pending_bound_checks:
        std::cell::RefCell<Option<Vec<(shiika_core::ty::TermTy, shiika_ast::LocationSpan)>>>,
    /// The type `Self` refers to while resolving type names in a module's
    /// instance method (or method requirement). `None` elsewhere.
    self_ty: std::cell::RefCell<Option<shiika_core::ty::TermTy>>,
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
        pending_bound_checks: Default::default(),
        self_ty: Default::default(),
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
        pending_bound_checks: Default::default(),
        self_ty: Default::default(),
        type_index,
        sk_types: Default::default(),
        imported_classes,
    };
    dict.index_program(defs, true)?;
    Ok(dict)
}

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Set the type which `Self` refers to while resolving type names.
    /// Returns the previous one (to be restored by the caller).
    pub fn set_self_ty(
        &self,
        ty: Option<shiika_core::ty::TermTy>,
    ) -> Option<shiika_core::ty::TermTy> {
        self.self_ty.replace(ty)
    }

    /// Returns `Self` of the module `fullname` (eg. `Self` whose upper bound
    /// is `Foo<X>` for `module Foo<X>`). `typarams` are the (bound-resolved)
    /// typarams of the module.
    pub fn module_self_ty(
        fullname: &TypeFullname,
        typarams: &[shiika_core::ty::TyParam],
    ) -> shiika_core::ty::TermTy {
        let tyargs = shiika_core::ty::typarams_to_typaram_refs(
            typarams,
            shiika_core::ty::TyParamKind::Class,
            false,
        )
        .into_iter()
        .map(|t| t.into_term_ty())
        .collect();
        shiika_core::ty::self_type(shiika_core::ty::LitTy::new(
            fullname.0.clone(),
            tyargs,
            false,
        ))
    }

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
