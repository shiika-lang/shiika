use crate::class_dict::ClassDict;
use anyhow::Result;
use shiika_core::names::*;
use std::collections::HashMap;

fn build_wtable(
    class_dict: &ClassDict,
    class: &ClassFullname,
    includes: &[Superclass],
) -> Result<WTable> {
    let wtable = HashMap::new();
    for module in includes {
        let sk_module = class_dict.get_module(module);
        let methods = resolve_module_methods(class_dict, class, sk_module)?;
        wtable.insert(sk_module.fullname(), methods);
    }
    Ok(WTable(wtable))
}

fn resolve_module_methods(
    class_dict: &ClassDict,
    class: &ClassFullname,
    sk_module: &SkModule,
) -> Result<Vec<MethodFullname>> {
    let resolved = vec![];
    for name in sk_module.base.method_names() {
        resolved.push(resolve_module_method(class_dict, class, name)?);
    }
    Ok(resolved)
}

fn resolve_module_method(
    class_dict: &ClassDict,
    class: &ClassFullname,
    name: &MethodFullname,
) -> Result<MethodFullname> {
    if let Some(sig) = sk_class.base.method_sigs.get(name.first_name) {
        チェック
    }
    // TODO: should look into its superclass?
    // If not found, use the default implementation
    Ok(name.clone())
}
