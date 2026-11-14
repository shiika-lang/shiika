use crate::class_dict::ClassDict;
use crate::error;
use anyhow::Result;
use shiika_core::names::*;
use shiika_core::ty::{self, TermTy, TyParam, TyParamKind};
use skc_hir::*;
use std::collections::HashMap;

/// Build a witness table for a Shiika class.
/// The table has a column for every module included by the class or its
/// ancestors, because the runtime looks up only the wtable of the
/// receiver's class.
/// `class_ty` is the type of the class (with its own typarams as type
/// arguments, eg. `Array<T>`).
/// A module implemented conditionally (in a `when` block) also has a column;
/// whether the class conforms to it is decided by the type checker.
pub fn build_wtable(
    class_dict: &ClassDict,
    class_ty: &TermTy,
    typarams: &[TyParam],
    instance_methods: &MethodSignatures,
    superclass: &Option<Supertype>,
    includes: &[Supertype],
    include_conds: &HashMap<String, Vec<WhenCond>>,
) -> Result<WTable> {
    let ancestors = ancestor_classes(class_dict, superclass);
    let mut wtable = HashMap::new();
    for (sup, level, self_ty) in
        all_includes(&ancestors, class_ty, typarams, includes, include_conds)
    {
        let sk_module = class_dict.get_module(&sup.erasure().to_module_fullname());
        if wtable.contains_key(&sk_module.fullname()) {
            continue;
        }
        let methods = resolve_module_methods(
            instance_methods,
            &ancestors,
            &self_ty,
            sk_module,
            &sup,
            level,
        )?;
        wtable.insert(sk_module.fullname(), methods);
    }
    Ok(WTable::new(wtable))
}

/// Returns the superclass chain, each specialized with the type parameters
/// of the class being indexed.
/// eg. given `class C<T> : B<Array<T>>` and `class B<U> : A<U>`,
/// returns `[(B, B<Array<T>>), (A, A<Array<T>>), (Object, Object)]`.
fn ancestor_classes<'a>(
    class_dict: &'a ClassDict,
    superclass: &Option<Supertype>,
) -> Vec<(&'a SkClass, Supertype)> {
    let mut v = vec![];
    let mut sup = superclass.clone();
    while let Some(s) = sup {
        let sk_class = class_dict.get_class(&s.erasure().to_class_fullname());
        sup = sk_class
            .superclass
            .as_ref()
            .map(|x| Supertype::from_ty(x.ty().substitute(s.type_args(), Default::default())));
        v.push((sk_class, s));
    }
    v
}

/// Returns the modules included by the class and its ancestors, specialized
/// with the type parameters of the class. Each module is paired with the
/// level of the class which includes it (0 = the class itself, 1 = its
/// superclass, ...) and `Self` of the module (the includer, see
/// `includer_ty`). Nearer ones come first.
fn all_includes(
    ancestors: &[(&SkClass, Supertype)],
    class_ty: &TermTy,
    typarams: &[TyParam],
    includes: &[Supertype],
    include_conds: &HashMap<String, Vec<WhenCond>>,
) -> Vec<(Supertype, usize, TermTy)> {
    let conds_of = |conds: &HashMap<String, Vec<WhenCond>>, m: &Supertype| {
        conds
            .get(&m.erasure().to_type_fullname().0)
            .cloned()
            .unwrap_or_default()
    };
    let mut v = includes
        .iter()
        .map(|m| {
            let self_ty = includer_ty(class_ty, typarams, &conds_of(include_conds, m));
            (m.clone(), 0, self_ty)
        })
        .collect::<Vec<_>>();
    for (i, (sk_class, sup)) in ancestors.iter().enumerate() {
        let generic_ty = ty::nonmeta(
            &sk_class.fullname().0,
            ty::typarams_to_typaram_refs(&sk_class.base.typarams, TyParamKind::Class, false)
                .into_iter()
                .map(|t| t.into_term_ty())
                .collect(),
        );
        for m in &sk_class.includes {
            let ty = m.ty().substitute(sup.type_args(), Default::default());
            let self_ty = includer_ty(
                &generic_ty,
                &sk_class.base.typarams,
                &conds_of(&sk_class.include_conds, m),
            )
            .substitute(sup.type_args(), Default::default());
            v.push((Supertype::from_ty(ty), i + 1, self_ty));
        }
    }
    v
}

/// Returns `Self` of a module implemented by `class_ty` (eg. `Array<T>`):
/// `class_ty` itself, but `T` is replaced with `X` when the module is
/// implemented in `when T == X`.
fn includer_ty(class_ty: &TermTy, typarams: &[TyParam], conds: &[WhenCond]) -> TermTy {
    if conds.iter().any(|c| c.kind == WhenCondKind::Eq) {
        let tyargs = crate::class_dict::when_body_tyargs(typarams, conds, true);
        class_ty.substitute_with_self(&tyargs)
    } else {
        class_ty.clone()
    }
}

/// Build a column of witness table whose key is `sk_module`
fn resolve_module_methods(
    instance_methods: &MethodSignatures,
    ancestors: &[(&SkClass, Supertype)],
    self_ty: &TermTy,
    sk_module: &SkModule,
    sup: &Supertype,
    level: usize,
) -> Result<Vec<MethodFullname>> {
    let mut resolved = vec![];
    for (mod_sig, _) in sk_module.base.method_sigs.to_ordered() {
        let required = sk_module.requirements.contains(mod_sig);
        resolved.push(resolve_module_method(
            instance_methods,
            ancestors,
            self_ty,
            mod_sig,
            sup,
            level,
            required,
        )?);
    }
    Ok(resolved)
}

/// Find the implementation of a module method in the same order as method
/// lookup: the classes nearer than the module (i.e. the class itself up to the
/// one which includes the module) come first, then the default implementation
/// in the module. A required method has no default implementation, so it may
/// also be implemented by the superclasses of the includer.
///
/// `Self` in the module's signature is replaced with the includer (the class
/// which has the `implement` block). The requirements are always defined in
/// the `implement` block (checked in indexing), so an implementation is never
/// found in an ancestor of the includer. An override in a subclass must accept
/// the includer (eg. `Dog#==(other: Animal)` for `Animal` which implements
/// `Equatable`) because it may be called via the includer.
fn resolve_module_method(
    instance_methods: &MethodSignatures,
    ancestors: &[(&SkClass, Supertype)],
    self_ty: &TermTy,
    mod_sig: &MethodSignature,
    sup: &Supertype,
    level: usize,
    required: bool,
) -> Result<MethodFullname> {
    let name = &mod_sig.fullname.first_name;
    // (signature, level of the class where it is found)
    let found =
        if let Some((sig, _)) = instance_methods.get(name) {
            Some((sig.clone(), 0))
        } else {
            let n = if required { ancestors.len() } else { level };
            ancestors[..n]
                .iter()
                .enumerate()
                .find_map(|(i, (sk_class, sup))| {
                    sk_class.base.method_sigs.get(name).map(|(sig, _)| {
                        (sig.specialize(sup.type_args(), Default::default()), i + 1)
                    })
                })
        };
    if let Some((sig, found_level)) = found {
        debug_assert!(found_level <= level);
        check_signature_matches(&sig, mod_sig, sup, self_ty)?;
        Ok(sig.fullname)
    } else {
        if required {
            return Err(error::program_error(&format!(
                "missing required method #{}",
                name,
            )));
        }
        // Use the default implementation
        Ok(mod_sig.fullname.clone())
    }
}

fn check_signature_matches(
    sig: &MethodSignature,
    mod_sig: &MethodSignature,
    sup: &Supertype,
    self_ty: &TermTy,
) -> Result<()> {
    let msig = mod_sig
        .specialize(sup.type_args(), Default::default())
        .replace_self(self_ty);
    if !sig.equivalent_to(&msig) {
        return Err(error::program_error(&format!(
            "signature does not match:\n  class': {}\n  module's: {}",
            sig, msig,
        )));
    }
    Ok(())
}
