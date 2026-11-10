use crate::class_dict::*;
use crate::error;
use crate::type_system;
use anyhow::Result;
use shiika_ast::LocationSpan;
use shiika_core::{names::*, ty, ty::*};
use skc_error::Label;
use skc_hir::*;

/// Replace `Self` of a module method called on a module-typed value
/// (eg. `x.eq?(y)` where `x: Eq`).
///
/// The actual class of the receiver is unknown, so:
/// - `Self` in the return type is the module type (the receiver's class
///   includes the module, whatever it is).
/// - `Self` in a parameter type could be satisfied only by `Never`, so the
///   method cannot be called practically. This is reported as an error.
/// - `Self` in other positions (eg. `Array<Self>` in the return type) is not
///   supported yet.
fn for_module_value(
    mut found: FoundMethod,
    module_ty: &TermTy,
    locs: &LocationSpan,
) -> Result<FoundMethod> {
    let sig = &found.sig;
    let in_params = sig.params.iter().any(|p| p.ty.contains_self_type());
    let nested_in_ret = sig.ret_ty.contains_self_type() && !sig.ret_ty.is_self_type();
    if in_params || nested_in_ret {
        let reason = if in_params {
            "it takes `Self' as a parameter type"
        } else {
            "`Self' appears in its return type (other than the return type itself)"
        };
        let msg = format!(
            "cannot call `{}' on a value of type {} because {}",
            &sig.fullname, module_ty, reason
        );
        let hint = format!(
            "the receiver may be of any class which includes {}; use a type parameter \
             (eg. `T: {}') to call it",
            module_ty, module_ty
        );
        let report = skc_error::build_report(msg, locs, |r, locs_span| {
            r.with_label(Label::new(locs_span).with_message(hint))
        });
        return Err(error::type_error(report));
    }
    found.replace_self(module_ty);
    Ok(found)
}

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Find a method in a class or module. Unlike `lookup_method`, does not lookup into superclass.
    pub fn find_method(
        &self,
        fullname: &TypeFullname,
        method_name: &MethodFirstname,
    ) -> Option<FoundMethod> {
        self.find_type(fullname)
            .and_then(|sk_type| self._find_method(sk_type, method_name))
    }

    fn _find_method(&self, sk_type: &SkType, method_name: &MethodFirstname) -> Option<FoundMethod> {
        match sk_type {
            SkType::Class(sk_class) => {
                sk_class.base.method_sigs.get(method_name).map(|(sig, _)| {
                    let call_type = if sig.is_virtual {
                        CallType::Virtual
                    } else {
                        CallType::Direct
                    };
                    FoundMethod::class(sk_type, sig.clone(), call_type)
                })
            }
            SkType::Module(sk_module) => sk_module
                .base
                .method_sigs
                .get(method_name)
                .map(|(sig, idx)| FoundMethod::module(sk_type, sig.clone(), *idx)),
        }
    }

    /// Like `find_method` but only returns the signature.
    pub fn find_method_sig(
        &self,
        fullname: &TypeFullname,
        method_name: &MethodFirstname,
    ) -> Option<MethodSignature> {
        self.find_method(fullname, method_name)
            .map(|found| found.sig)
    }

    /// Similar to find_method, but lookup into superclass if not in the class.
    /// Returns the class where the method is found as a `TermTy`.
    /// Returns Err if not found.
    pub fn lookup_method(
        &self,
        receiver_type: &TermTy,
        method_name: &MethodFirstname,
        // For error messages
        locs: &LocationSpan,
    ) -> Result<FoundMethod> {
        self.lookup_method_(receiver_type, receiver_type, false, method_name, locs)
    }

    pub fn try_lookup_method(
        &self,
        receiver_type: &TermTy,
        method_name: &MethodFirstname,
    ) -> Option<FoundMethod> {
        self.lookup_method_(
            receiver_type,
            receiver_type,
            false,
            method_name,
            &LocationSpan::todo(),
        )
        .ok()
    }

    // `receiver_type` is for error message.
    // `in_superclass` is true if `current_type` is a superclass of the receiver.
    //
    // `Self` in the signature of a module method is replaced as follows:
    // - Found via a class: `Self` is the class which includes the module
    //   (eg. `Animal` for `dog.neq?(...)` where `Animal : Eq`)
    // - Found via a typaram whose upper bound is the module (including `Self`
    //   itself): `Self` is the typaram
    // - Found via a module-typed value: see `for_module_value`
    fn lookup_method_(
        &self,
        receiver_type: &TermTy,
        current_type: &TermTy,
        in_superclass: bool,
        method_name: &MethodFirstname,
        locs: &LocationSpan,
    ) -> Result<FoundMethod> {
        let upper_bound; // Extends the lifetime beyond the `match`
        let (erasure, class_tyargs) = match &current_type.body {
            TyBody::TyRaw(LitTy { type_args, .. }) => {
                (current_type.erasure(), type_args.as_slice())
            }
            TyBody::TyPara(_) => {
                // Methods on a typaram-typed value are looked up via its upper bound
                upper_bound = current_type.upper_bound();
                (upper_bound.erasure(), upper_bound.type_args.as_slice())
            }
        };
        let sk_type = self.get_type(&erasure.to_type_fullname());
        // What `Self` of the module methods found below is replaced with
        let module_self = if current_type.is_typaram_ref() {
            Some(current_type.clone())
        } else {
            None
        };
        // True if the receiver is a module-typed value (eg. `x: Eq`)
        let module_value = !current_type.is_typaram_ref() && sk_type.is_module();
        if let Some(mut found) = self.find_method(&sk_type.base().fullname(), method_name) {
            if module_value {
                return for_module_value(found, current_type, locs);
            }
            if in_superclass {
                // Found in a superclass (eg. `G<String>` of `class H : G<String>`).
                // Its type arguments are already expressed in terms of the receiver.
                found.specialize(class_tyargs, Default::default());
            }
            if let Some(t) = &module_self {
                found.replace_self(t);
            }
            return Ok(found);
        }
        match sk_type {
            SkType::Class(sk_class) => {
                // Look up in included modules
                for modinfo in &sk_class.includes {
                    if let Some(mut found) =
                        self.find_method(&modinfo.erasure().to_type_fullname(), method_name)
                    {
                        let mod_tyargs = sk_class.specialize_module(modinfo, class_tyargs);
                        found.specialize(&mod_tyargs, Default::default());
                        // `Self` is this class (the includer)
                        let includer = ty::new(&erasure.base_name, class_tyargs.to_vec(), false);
                        found.replace_self(&includer);
                        return Ok(found);
                    }
                }
                // Look up in superclass
                if let Some(super_ty) = &sk_class.specialized_superclass(class_tyargs) {
                    return self.lookup_method_(
                        receiver_type,
                        &super_ty.to_term_ty(),
                        true,
                        method_name,
                        locs,
                    );
                }
            }
            SkType::Module(sk_module) => {
                // Look up in prerequisite modules
                if let Some(mut found) =
                    self.lookup_in_module_supers(sk_module, class_tyargs, method_name)
                {
                    // `Self` of a prerequisite module is the same as that of `sk_module`
                    if module_value {
                        return for_module_value(found, current_type, locs);
                    }
                    if let Some(t) = &module_self {
                        found.replace_self(t);
                    }
                    return Ok(found);
                }
                // Lookup in Object (the top type)
                return self.lookup_method_(
                    receiver_type,
                    &ty::raw("Object"),
                    false,
                    method_name,
                    locs,
                );
            }
        }
        Err(error::method_not_found(
            format!(
                "method {:?} not found on {:?}",
                method_name, receiver_type.fullname
            ),
            locs,
        ))
    }

    /// Look up a method in the prerequisite modules of `sk_module`
    /// (depth-first, in the declared order). `tyargs` are the type
    /// arguments of `sk_module`.
    fn lookup_in_module_supers(
        &self,
        sk_module: &SkModule,
        tyargs: &[TermTy],
        method_name: &MethodFirstname,
    ) -> Option<FoundMethod> {
        for sup in sk_module.specialized_supers(tyargs) {
            let fullname = sup.erasure().to_type_fullname();
            if let Some(mut found) = self.find_method(&fullname, method_name) {
                found.specialize(sup.type_args(), Default::default());
                return Some(found);
            }
            let sup_module = self.get_module(&sup.erasure().to_module_fullname());
            if let Some(found) =
                self.lookup_in_module_supers(sup_module, sup.type_args(), method_name)
            {
                return Some(found);
            }
        }
        None
    }

    /// Return the class/module of the specified name, if any
    pub fn find_type(&self, fullname: &TypeFullname) -> Option<&SkType> {
        self.sk_types
            .types
            .get(fullname)
            .or_else(|| self.imported_classes.types.get(fullname))
    }

    /// Return the type parameters of the specified type.
    /// Unlike `find_type`, this also works for types not indexed yet
    /// (by looking up `self.type_index`).
    pub fn typarams_of(&self, fullname: &TypeFullname) -> &[TyParam] {
        if let Some(typarams) = self.type_index.get(fullname) {
            return typarams;
        }
        if let Some(sk_type) = self.find_type(fullname) {
            return &sk_type.base().typarams;
        }
        &[]
    }

    /// Return the class of the specified name, if any
    pub fn lookup_class(&self, class_fullname: &ClassFullname) -> Option<&SkClass> {
        self.sk_types
            .types
            .get(&class_fullname.to_type_fullname())
            .or_else(|| {
                self.imported_classes
                    .types
                    .get(&class_fullname.to_type_fullname())
            })
            .and_then(|sk_type| {
                if let SkType::Class(c) = sk_type {
                    Some(c)
                } else {
                    None
                }
            })
    }

    /// Return the module of the specified name, if any
    pub fn lookup_module(&self, module_fullname: &ModuleFullname) -> Option<&SkModule> {
        let name = module_fullname.to_type_fullname();
        self.sk_types
            .types
            .get(&name)
            .or_else(|| self.imported_classes.types.get(&name))
            .and_then(|sk_type| {
                if let SkType::Module(m) = sk_type {
                    Some(m)
                } else {
                    None
                }
            })
    }

    /// Find a type. Panic if not found
    pub fn get_type(&self, fullname: &TypeFullname) -> &SkType {
        self.find_type(fullname)
            .unwrap_or_else(|| panic!("[BUG] class/module `{}' not found", &fullname.0))
    }

    /// Find a type. Panic if not found
    pub fn get_type_mut(&mut self, fullname: &TypeFullname) -> &mut SkType {
        self.sk_types
            .types
            .get_mut(fullname)
            .unwrap_or_else(|| panic!("[BUG] class/module `{}' not found", &fullname.0))
    }

    /// Find a class. Panic if not found
    pub fn get_class(&self, class_fullname: &ClassFullname) -> &SkClass {
        self.lookup_class(class_fullname)
            .unwrap_or_else(|| panic!("[BUG] class `{}' not found", &class_fullname.0))
    }

    /// Find a module. Panic if not found
    pub fn get_module(&self, module_fullname: &ModuleFullname) -> &SkModule {
        self.lookup_module(module_fullname)
            .unwrap_or_else(|| panic!("[BUG] module `{}' not found", &module_fullname.0))
    }

    /// Find a class. Panic if not found
    pub fn get_class_mut(&mut self, class_fullname: &ClassFullname) -> &mut SkClass {
        if let Some(sk_type) = self
            .sk_types
            .types
            .get_mut(&class_fullname.to_type_fullname())
        {
            if let SkType::Class(c) = sk_type {
                c
            } else {
                panic!("[BUG] `{}' is not a class", class_fullname)
            }
        } else if self
            .imported_classes
            .types
            .contains_key(&class_fullname.to_type_fullname())
        {
            panic!("[BUG] cannot get_mut imported class `{}'", class_fullname)
        } else {
            panic!("[BUG] class `{}' not found", class_fullname)
        }
    }

    /// Returns supertype of `ty` (except it is `Object`)
    pub fn supertype(&self, ty: &TermTy) -> Option<LitTy> {
        match &ty.body {
            TyBody::TyPara(TyParamRef { upper_bound, .. }) => Some(upper_bound.clone()),
            _ => self
                .get_class(&ty.erasure().to_class_fullname())
                .superclass
                .as_ref()
                .map(|scls| scls.ty().substitute(ty.tyargs(), &[])),
        }
    }

    /// Returns the nearest common ancestor of the classes
    pub fn nearest_common_ancestor(&self, ty1: &TermTy, ty2: &TermTy) -> Option<TermTy> {
        type_system::subtyping::nearest_common_ancestor(self, ty1, ty2)
    }

    /// Return true if `ty1` conforms to `ty2` i.e.
    /// an object of the type `ty1` is included in the set of objects represented by the type `ty2`
    pub fn conforms(&self, ty1: &TermTy, ty2: &TermTy) -> bool {
        type_system::subtyping::conforms(self, ty1, ty2)
    }

    pub fn find_ivar(&self, classname: &ClassFullname, ivar_name: &str) -> Option<&SkIVar> {
        let class = self.lookup_class(classname).unwrap_or_else(|| {
            panic!(
                "[BUG] finding ivar `{}' but the class '{}' not found",
                ivar_name, &classname
            )
        });
        class.ivars.get(ivar_name)
    }

    /// Returns instance variables of the superclass of `classname`
    pub fn superclass_ivars(&self, sup: &Option<Supertype>) -> Option<SkIVars> {
        sup.as_ref().map(|scls| {
            let ty = scls.ty();
            let ivars = &self.get_class(&ty.erasure().to_class_fullname()).ivars;
            ivars
                .iter()
                .map(|(name, ivar)| (name.clone(), ivar.substitute(&ty.type_args)))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::class_dict::*;
    use anyhow::{Context, Result};
    use shiika_core::ty;
    use shiika_parser::{Parser, SourceFile};
    use skc_mir::LibraryExports;
    use std::path::PathBuf;

    /// Load exports.json of the core package.
    /// Requires the core package built (`shiika build packages/core`),
    /// like tests/erroneous.rs does.
    fn load_core_exports() -> Result<LibraryExports> {
        let shiika_work = std::env::var("SHIIKA_WORK")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap()).join(".shiika"));
        let packages_dir = shiika_work.join("packages");
        let core_dir = std::fs::read_dir(&packages_dir)
            .with_context(|| format!("could not read {}", packages_dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("core-"))
                    .unwrap_or(false)
            })
            .context("core package is not built yet; run `shiika build packages/core` first")?;
        let json = std::fs::read_to_string(core_dir.join("lib/exports.json"))?;
        Ok(serde_json::from_str(&json)?)
    }

    fn test_class_dict<F>(s: &str, f: F) -> Result<()>
    where
        F: FnOnce(ClassDict),
    {
        let core = load_core_exports()?;
        let src = SourceFile::new(PathBuf::from("(test)"), s.to_string());
        let ast = Parser::parse_files(&[src])?;
        let defs = ast.defs();
        let type_index = crate::type_index::create(&defs, &Default::default(), &core.sk_types);
        let class_dict = crate::class_dict::create(&defs, type_index, &core.sk_types)?;
        f(class_dict);
        Ok(())
    }

    #[test]
    fn test_supertype_default() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            assert_eq!(
                class_dict
                    .supertype(&ty::ary(ty::raw("Int")))
                    .map(|lit_ty| lit_ty.to_term_ty()),
                Some(ty::raw("Object"))
            )
        })
    }

    #[test]
    fn test_supertype_gen() -> Result<()> {
        let src = "
          base class MyList<X>
          end
          class A<S, T> : MyList<T>
          end
        ";
        test_class_dict(src, |class_dict| {
            assert_eq!(
                class_dict
                    .supertype(&ty::spe("A", vec![ty::raw("Int"), ty::raw("Bool")]))
                    .map(|lit_ty| lit_ty.to_term_ty()),
                Some(ty::spe("MyList", vec![ty::raw("Bool")]))
            )
        })
    }

    #[test]
    fn test_conforms_some() -> Result<()> {
        let src = "
            base class MyMaybe<T>; end
            class MySome<T> : MyMaybe<T>; end
        ";
        test_class_dict(src, |class_dict| {
            let some_int = ty::spe("MySome", vec![ty::raw("Int")]);
            let maybe_int = ty::spe("MyMaybe", vec![ty::raw("Int")]);
            assert!(class_dict.conforms(&some_int, &maybe_int));
        })
    }

    #[test]
    fn test_conforms_none() -> Result<()> {
        let src = "
            base class MyMaybe<T>; end
            class MyNone : MyMaybe<Never>; end
        ";
        test_class_dict(src, |class_dict| {
            let none = ty::raw("MyNone");
            let maybe_int = ty::spe("MyMaybe", vec![ty::raw("Int")]);
            assert!(class_dict.conforms(&none, &maybe_int));
        })
    }

    #[test]
    fn test_conforms_covariant() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let m_int = ty::spe("Maybe", vec![ty::raw("Int")]);
            let m_never = ty::spe("Maybe", vec![ty::raw("Never")]);
            assert!(class_dict.conforms(&m_never, &m_int));
        })
    }

    #[test]
    fn test_conforms_invalid() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let a = ty::raw("Int");
            let b = ty::raw("Bool");
            assert!(!class_dict.conforms(&a, &b));
        })
    }

    #[test]
    fn test_conforms_not() -> Result<()> {
        let src = "
            base class MyList<X>; end
            class A : MyList<Int>; end
            class B : MyList<Bool>; end
        ";
        test_class_dict(src, |class_dict| {
            let a = ty::raw("A");
            let b = ty::raw("B");
            assert!(!class_dict.conforms(&a, &b));
        })
    }

    #[test]
    fn test_conforms_void_func() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let a = ty::spe("Fn0", vec![ty::raw("Int")]);
            let b = ty::spe("Fn0", vec![ty::raw("Void")]);
            assert!(class_dict.conforms(&a, &b));
        })
    }

    #[test]
    fn test_nearest_common_ancestor__some() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let a = ty::raw("Maybe::None");
            let b = ty::spe("Maybe::Some", vec![ty::raw("Int")]);
            let c = class_dict.nearest_common_ancestor(&a, &b);
            assert_eq!(c, Some(ty::spe("Maybe", vec![ty::raw("Int")])));
            let d = class_dict.nearest_common_ancestor(&b, &a);
            assert_eq!(d, Some(ty::spe("Maybe", vec![ty::raw("Int")])));
        })
    }

    #[test]
    fn test_nearest_common_ancestor__some_object() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let a = ty::raw("Int");
            let b = ty::raw("Object");
            let c = class_dict.nearest_common_ancestor(&a, &b);
            assert_eq!(c, Some(ty::raw("Object")));
        })
    }

    #[test]
    fn test_nearest_common_ancestor__none() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let a = ty::raw("Int");
            let b = ty::spe("Array", vec![ty::raw("Int")]);
            let c = class_dict.nearest_common_ancestor(&a, &b);
            assert_eq!(c, None);
        })
    }
}
