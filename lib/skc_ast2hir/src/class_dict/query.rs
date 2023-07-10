use crate::class_dict::*;
use crate::error;
use crate::type_system;
use anyhow::Result;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::*;

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
            SkType::Class(sk_class) => sk_class
                .base
                .method_sigs
                .get(method_name)
                .map(|(sig, _)| FoundMethod::class(sk_type, sig.clone())),
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
        method_tyargs: &[TermTy],
    ) -> Result<FoundMethod> {
        self.lookup_method_(receiver_type, receiver_type, method_name, method_tyargs)
    }

    // `receiver_type` is for error message.
    fn lookup_method_(
        &self,
        receiver_type: &TermTy,
        current_type: &TermTy,
        method_name: &MethodFirstname,
        method_tyargs: &[TermTy],
    ) -> Result<FoundMethod> {
        let (erasure, class_tyargs) = match &current_type.body {
            TyBody::TyRaw(LitTy { type_args, .. }) => {
                (current_type.erasure(), type_args.as_slice())
            }
            TyBody::TyPara(_) => (Erasure::nonmeta("Object"), Default::default()),
        };
        let sk_type = self.get_type(&erasure.to_type_fullname());
        if let Some(found) = self.find_method(&sk_type.base().fullname(), method_name) {
            if method_tyargs.len() > 0 && method_tyargs.len() != found.sig.typarams.len() {
                return Err(error::type_error(format!(
                    "wrong number of type arguments, expected: {:?} got: {:?}",
                    &found.sig.typarams.len(),
                    method_tyargs.len(),
                )));
            }

            return Ok(specialized_version(
                found,
                receiver_type,
                class_tyargs,
                method_tyargs,
            ));
        }
        match sk_type {
            SkType::Class(sk_class) => {
                // Look up in included modules
                for modinfo in &sk_class.includes {
                    if let Some(mut found) =
                        self.find_method(&modinfo.erasure().to_type_fullname(), method_name)
                    {
                        let mod_tyargs = sk_class.specialize_module(modinfo, &class_tyargs);
                        found.specialize(&mod_tyargs, method_tyargs);
                        return Ok(found);
                    }
                }
                // Look up in superclass
                if let Some(super_ty) = &sk_class.specialized_superclass(&class_tyargs) {
                    return self.lookup_method_(
                        receiver_type,
                        &super_ty.to_term_ty(),
                        method_name,
                        method_tyargs,
                    );
                }
            }
            SkType::Module(_) => {
                // TODO: Look up in supermodule, once it's implemented
                return self.lookup_method_(
                    receiver_type,
                    &ty::raw("Object"),
                    method_name,
                    method_tyargs,
                );
            }
        }
        Err(error::program_error(&format!(
            "method {:?} not found on {:?}",
            method_name, receiver_type.fullname
        )))
    }

    /// Return the class/module of the specified name, if any
    pub fn find_type(&self, fullname: &TypeFullname) -> Option<&SkType> {
        self.sk_types
            .0
            .get(fullname)
            .or_else(|| self.imported_classes.0.get(fullname))
    }

    /// Return the class of the specified name, if any
    pub fn lookup_class(&self, class_fullname: &ClassFullname) -> Option<&SkClass> {
        self.sk_types
            .0
            .get(&class_fullname.to_type_fullname())
            .or_else(|| {
                self.imported_classes
                    .0
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
            .0
            .get(&name)
            .or_else(|| self.imported_classes.0.get(&name))
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
        if let Some(sk_type) = self.sk_types.0.get_mut(&class_fullname.to_type_fullname()) {
            if let SkType::Class(c) = sk_type {
                c
            } else {
                panic!("[BUG] `{}' is not a class", class_fullname)
            }
        } else if self
            .imported_classes
            .0
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
    pub fn superclass_ivars(&self, classname: &ClassFullname) -> Option<SkIVars> {
        self.get_class(classname).superclass.as_ref().map(|scls| {
            let ty = scls.ty();
            let ivars = &self.get_class(&ty.erasure().to_class_fullname()).ivars;
            ivars
                .iter()
                .map(|(name, ivar)| (name.clone(), ivar.substitute(&ty.type_args)))
                .collect()
        })
    }
}

fn specialized_version(
    mut found: FoundMethod,
    receiver_ty: &TermTy,
    class_tyargs: &[TermTy],
    method_tyargs: &[TermTy],
) -> FoundMethod {
    if found.sig.fullname.first_name.0 == "new"
        && receiver_ty.is_metaclass()
        && receiver_ty.has_type_args()
    {
        // Special handling for `.new`.
        // self:    `#new<A0M,B1M>(a: A0M, b: B1M) -> Pair<A0M,B1M>`
        // returns: `#new(a: A0C, b: B1C) -> Pair<A0C,B1C>`
        let sig2 = found.sig.specialize(Default::default(), class_tyargs);
        FoundMethod {
            sig: MethodSignature {
                typarams: Default::default(),
                ..sig2
            },
            ..found
        }
    } else {
        found.specialize(class_tyargs, method_tyargs);
        found
    }
}

#[cfg(test)]
mod tests {
    use crate::class_dict::*;
    use crate::error;
    use crate::ty;
    use anyhow::Result;

    fn test_class_dict<F>(s: &str, f: F) -> Result<()>
    where
        F: FnOnce(ClassDict),
    {
        let core = crate::runner::load_builtin_exports()?;
        let ast = crate::parser::Parser::parse(s)?;
        let class_dict = crate::hir::class_dict::create(&ast, Default::default(), &core.sk_types)?;
        f(class_dict);
        Ok(())
    }

    #[test]
    fn test_supertype_default() -> Result<()> {
        let src = "";
        test_class_dict(src, |class_dict| {
            assert_eq!(
                class_dict.supertype(&ty::ary(ty::raw("Int"))),
                Some(ty::raw("Object"))
            )
        })
    }

    #[test]
    fn test_supertype_gen() -> Result<()> {
        let src = "
          class A<S, T> : Array<T>
          end
        ";
        test_class_dict(src, |class_dict| {
            assert_eq!(
                class_dict.supertype(&ty::spe("A", vec![ty::raw("Int"), ty::raw("Bool")])),
                Some(ty::ary(ty::raw("Bool")))
            )
        })
    }

    #[test]
    fn test_conforms_some() -> Result<()> {
        let src = "
            class MyMaybe<T>; end
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
            class MyMaybe<T>; end
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
            class A : Array<Int>; end
            class B : Array<Bool>; end
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
