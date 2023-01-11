use crate::class_dict::ClassDict;
use crate::error;
use anyhow::Result;
use shiika_core::names::*;
use skc_hir::*;
use std::collections::HashMap;

/// Build a witness table for a Shiika class
pub fn build_wtable(
    class_dict: &ClassDict,
    instance_methods: &MethodSignatures,
    includes: &[Supertype],
) -> Result<WTable> {
    let mut wtable = HashMap::new();
    for sup in includes {
        let sk_module = class_dict.get_module(&sup.erasure().to_module_fullname());
        let methods = resolve_module_methods(instance_methods, sk_module, sup)?;
        wtable.insert(sk_module.fullname(), methods);
    }
    Ok(WTable::new(wtable))
}

/// Build a column of witness table whose key is `sk_module`
fn resolve_module_methods(
    instance_methods: &MethodSignatures,
    sk_module: &SkModule,
    sup: &Supertype,
) -> Result<Vec<MethodFullname>> {
    let mut resolved = vec![];
    for (mod_sig, _) in sk_module.base.method_sigs.to_ordered() {
        let required = sk_module.requirements.contains(mod_sig);
        resolved.push(resolve_module_method(
            instance_methods,
            mod_sig,
            sup,
            required,
        )?);
    }
    Ok(resolved)
}

fn resolve_module_method(
    instance_methods: &MethodSignatures,
    mod_sig: &MethodSignature,
    sup: &Supertype,
    required: bool,
) -> Result<MethodFullname> {
    if let Some((sig, _)) = instance_methods.get(&mod_sig.fullname.first_name) {
        check_signature_matches(sig, mod_sig, sup)?;
        Ok(sig.fullname.clone())
    } else {
        if required {
            return Err(error::program_error(&format!(
                "missing required method #{}",
                &mod_sig.fullname.first_name,
            )));
        }

        // TODO: should look into the superclass?

        // If not found, use the default implementation
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
