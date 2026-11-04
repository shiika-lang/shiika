use crate::class_dict::ClassDict;
use crate::error;
use anyhow::Result;
use shiika_core::names::*;
use skc_hir::*;
use std::collections::HashMap;

/// Build a witness table for a Shiika class.
/// The table has a column for every module included by the class or its
/// ancestors, because the runtime looks up only the wtable of the
/// receiver's class.
pub fn build_wtable(
    class_dict: &ClassDict,
    instance_methods: &MethodSignatures,
    superclass: &Option<Supertype>,
    includes: &[Supertype],
) -> Result<WTable> {
    let ancestors = ancestor_classes(class_dict, superclass);
    let mut wtable = HashMap::new();
    for (sup, level) in all_includes(&ancestors, includes) {
        let sk_module = class_dict.get_module(&sup.erasure().to_module_fullname());
        if wtable.contains_key(&sk_module.fullname()) {
            continue;
        }
        let methods = resolve_module_methods(instance_methods, &ancestors, sk_module, &sup, level)?;
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
/// superclass, ...). Nearer ones come first.
fn all_includes(
    ancestors: &[(&SkClass, Supertype)],
    includes: &[Supertype],
) -> Vec<(Supertype, usize)> {
    let mut v = includes.iter().map(|m| (m.clone(), 0)).collect::<Vec<_>>();
    for (i, (sk_class, sup)) in ancestors.iter().enumerate() {
        for m in &sk_class.includes {
            let ty = m.ty().substitute(sup.type_args(), Default::default());
            v.push((Supertype::from_ty(ty), i + 1));
        }
    }
    v
}

/// Build a column of witness table whose key is `sk_module`
fn resolve_module_methods(
    instance_methods: &MethodSignatures,
    ancestors: &[(&SkClass, Supertype)],
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
fn resolve_module_method(
    instance_methods: &MethodSignatures,
    ancestors: &[(&SkClass, Supertype)],
    mod_sig: &MethodSignature,
    sup: &Supertype,
    level: usize,
    required: bool,
) -> Result<MethodFullname> {
    let name = &mod_sig.fullname.first_name;
    let found = if let Some((sig, _)) = instance_methods.get(name) {
        Some(sig.clone())
    } else {
        let n = if required { ancestors.len() } else { level };
        ancestors[..n].iter().find_map(|(sk_class, sup)| {
            sk_class
                .base
                .method_sigs
                .get(name)
                .map(|(sig, _)| sig.specialize(sup.type_args(), Default::default()))
        })
    };
    if let Some(sig) = found {
        check_signature_matches(&sig, mod_sig, sup)?;
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
) -> Result<()> {
    let msig = mod_sig.specialize(sup.type_args(), Default::default());
    if !sig.equivalent_to(&msig) {
        return Err(error::program_error(&format!(
            "signature does not match:\n  class': {}\n  module's: {}",
            sig, msig,
        )));
    }
    Ok(())
}
