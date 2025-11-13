use skc_hir::{SkMethods, SkTypes};

/// Before generating MIR, set sig.asyncness to Async  for methods that must be treated as async regardless of their actual implementation.
/// - Methods of base classes
/// - Methods of modules
/// - Methods that override inherited methods
pub fn run(sk_types: &mut SkTypes, _sk_methods: &mut SkMethods, imported_types: &SkTypes) {
    // Buffer changes to avoid borrow checker issues
    let mut fix_types = vec![];
    let mut fix_methods = vec![];

    for (type_fullname, sk_type) in &sk_types.types {
        match sk_type {
            skc_hir::SkType::Module(_) => {
                // All methods of modules must be async
                fix_types.push(type_fullname.clone());
            }
            skc_hir::SkType::Class(sk_class) => {
                if sk_class.inheritable {
                    // All methods of inheritable classes must be async
                    fix_types.push(type_fullname.clone());
                } else {
                    // Check if any methods override inherited methods or are from modules
                    for sig in sk_class.base.method_sigs.iter() {
                        let method_name = &sig.fullname.first_name;
                        let mut should_be_async = false;

                        // Check if method overrides a superclass method
                        if let Some(superclass_ref) = &sk_class.superclass {
                            let superclass_fullname =
                                superclass_ref.base_fullname().to_type_fullname();
                            if let Some(superclass_type) = sk_types
                                .get_type(&superclass_fullname)
                                .or_else(|| imported_types.get_type(&superclass_fullname))
                            {
                                if superclass_type.base().method_sigs.contains_key(method_name) {
                                    should_be_async = true;
                                }
                            }
                        }

                        // Check if method is from an included module
                        if !should_be_async {
                            for incl in &sk_class.includes {
                                let module_fullname = incl.base_fullname().to_type_fullname();
                                if let Some(module_type) = sk_types
                                    .get_type(&module_fullname)
                                    .or_else(|| imported_types.get_type(&module_fullname))
                                {
                                    if module_type.base().method_sigs.contains_key(method_name) {
                                        should_be_async = true;
                                        break;
                                    }
                                }
                            }
                        }

                        if should_be_async {
                            fix_methods.push((type_fullname.clone(), method_name.clone()));
                        }
                    }
                }
            }
        }
    }

    // Now apply the asyncness changes
    for type_fullname in &fix_types {
        let sk_type = sk_types.types.get_mut(type_fullname).unwrap();
        for sig in sk_type.base_mut().method_sigs.iter_mut() {
            sig.asyncness = skc_hir::Asyncness::Async;
        }
    }
    for (type_fullname, method_name) in &fix_methods {
        let sk_type = sk_types.types.get_mut(type_fullname).unwrap();
        let (sig, _) = sk_type.base_mut().method_sigs.get_mut(method_name).unwrap();
        sig.asyncness = skc_hir::Asyncness::Async;
    }
}
