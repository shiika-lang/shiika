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
    includes: &[Superclass],
) -> Result<WTable> {
    let mut wtable = HashMap::new();
    for sup in includes {
        let sk_module = class_dict.get_module(&sup.erasure().to_module_fullname());
        let methods = resolve_module_methods(instance_methods, sk_module)?;
        wtable.insert(sk_module.fullname(), methods);
    }
    Ok(WTable::new(wtable))
}

/// Build a column of witness table whose key is `sk_module`
fn resolve_module_methods(
    instance_methods: &MethodSignatures,
    sk_module: &SkModule,
) -> Result<Vec<MethodFullname>> {
    let mut resolved = vec![];
    for mod_sig in &sk_module.requirements {
        resolved.push(resolve_module_method(instance_methods, mod_sig)?);
    }
    for (mod_sig, _) in sk_module.base.method_sigs.to_ordered() {
        resolved.push(resolve_module_method(instance_methods, mod_sig)?);
    }
    Ok(resolved)
}

fn resolve_module_method(
    instance_methods: &MethodSignatures,
    mod_sig: &MethodSignature,
) -> Result<MethodFullname> {
    if let Some((sig, _)) = instance_methods.get(&mod_sig.fullname.first_name) {
        check_signature_matches(sig, mod_sig)?;
        return Ok(sig.fullname.clone());
    }

    // TODO: should look into the superclass?

    // If not found, use the default implementation
    Ok(mod_sig.fullname.clone())
}

fn check_signature_matches(sig: &MethodSignature, mod_sig: &MethodSignature) -> Result<()> {
    if !sig.equivalent_to(&mod_sig) {
        return Err(error::program_error(&format!(
            "signature does not match (class': {:?}, module's: {:?})",
            sig, mod_sig,
        )));
    }
    Ok(())
}
