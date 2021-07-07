use crate::error;
use crate::error::*;
use crate::hir::class_dict::class_dict::ClassDict;
use crate::hir::*;
use crate::names::*;
use crate::ty::*;

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Find a method from class name and first name
    pub fn find_method(
        &self,
        class_fullname: &ClassFullname,
        method_name: &MethodFirstname,
    ) -> Option<&MethodSignature> {
        self.lookup_class(class_fullname)
            .and_then(|class| class.method_sigs.get(method_name))
    }

    /// Similar to find_method, but lookup into superclass if not in the class.
    /// Returns Err if not found.
    pub fn lookup_method(
        &self,
        receiver_class: &TermTy,
        method_name: &MethodFirstname,
        method_tyargs: &[TermTy],
    ) -> Result<(MethodSignature, ClassFullname), Error> {
        self.lookup_method_(receiver_class, receiver_class, method_name, method_tyargs)
    }

    fn lookup_method_(
        &self,
        receiver_class: &TermTy,
        class: &TermTy,
        method_name: &MethodFirstname,
        method_tyargs: &[TermTy],
    ) -> Result<(MethodSignature, ClassFullname), Error> {
        let ty_obj = ty::raw("Object");
        let (class, class_tyargs) = match &class.body {
            TyBody::TyRaw | TyBody::TyMeta { .. } | TyBody::TyClass => (class, Default::default()),
            TyBody::TySpe { type_args, .. } | TyBody::TySpeMeta { type_args, .. } => {
                let base_cls = &self.get_class(&class.base_class_name()).instance_ty;
                (base_cls, type_args.as_slice())
            }
            TyBody::TyParamRef { .. } => (&ty_obj, Default::default()),
            _ => todo!("{}", class),
        };
        if let Some(sig) = self.find_method(&class.fullname, method_name) {
            Ok((
                sig.specialize(class_tyargs, method_tyargs),
                class.fullname.clone(),
            ))
        } else {
            // Look up in superclass
            let sk_class = self.get_class(&class.erasure());
            if let Some(superclass) = &sk_class.superclass {
                self.lookup_method_(receiver_class, superclass.ty(), method_name, method_tyargs)
            } else {
                Err(error::program_error(&format!(
                    "method {:?} not found on {:?}",
                    method_name, receiver_class.fullname
                )))
            }
        }
    }

    /// Return the class of the specified name, if any
    pub fn lookup_class(&self, class_fullname: &ClassFullname) -> Option<&SkClass> {
        self.sk_classes
            .get(class_fullname)
            .or_else(|| self.imported_classes.get(class_fullname))
    }

    /// Returns if there is a class of the given name
    /// Find a class. Panic if not found
    pub fn get_class(&self, class_fullname: &ClassFullname) -> &SkClass {
        self.lookup_class(class_fullname)
            .unwrap_or_else(|| panic!("[BUG] class `{}' not found", &class_fullname.0))
    }

    /// Find a class. Panic if not found
    pub fn get_class_mut(&mut self, class_fullname: &ClassFullname) -> &mut SkClass {
        if let Some(c) = self.sk_classes.get_mut(class_fullname) {
            c
        } else if self.imported_classes.contains_key(class_fullname) {
            panic!("[BUG] cannot get_mut imported class `{}'", class_fullname)
        } else {
            panic!("[BUG] class `{}' not found", class_fullname)
        }
    }

    /// Return true if there is a class of the name
    pub fn class_exists(&self, fullname: &str) -> bool {
        self.lookup_class(&class_fullname(fullname)).is_some()
    }

    /// Returns supertype of `ty` (except it is `Object`)
    pub fn supertype(&self, ty: &TermTy) -> Option<TermTy> {
        self.get_class(&ty.erasure())
            .superclass
            .as_ref()
            .map(|scls| scls.ty().substitute(ty.tyargs(), &[]))
    }

    /// Return ancestor types of `ty`, including itself.
    fn ancestor_types(&self, ty: &TermTy) -> Vec<TermTy> {
        let mut v = vec![];
        let mut t = Some(ty.clone());
        while let Some(tt) = t {
            t = self.supertype(&tt);
            v.push(tt);
        }
        v
    }

    /// Return the nearest common ancestor of the classes
    pub fn nearest_common_ancestor(&self, ty1: &TermTy, ty2: &TermTy) -> TermTy {
        let ancestors1 = self.ancestor_types(ty1);
        let ancestors2 = self.ancestor_types(ty2);
        for t2 in ancestors2 {
            if let Some(eq) = ancestors1.iter().find(|t1| t1.equals_to(&t2)) {
                return eq.clone();
            }
        }
        panic!("[BUG] nearest_common_ancestor_type not found");
    }

    /// Return true if ty1 is an descendant of ty2
    /// Return value is unspecified when ty1 == ty2
    pub fn is_descendant(&self, ty1: &TermTy, ty2: &TermTy) -> bool {
        let mut t = Some(ty1.clone());
        while let Some(tt) = t {
            if tt == *ty2 {
                return true;
            }
            t = self.supertype(&tt);
        }
        false
    }

    /// Return true if `ty1` conforms to `ty2` i.e.
    /// an object of the type `ty1` is included in the set of objects represented by the type `ty2`
    pub fn conforms(&self, ty1: &TermTy, ty2: &TermTy) -> bool {
        // `Never` is bottom type (i.e. subclass of any class)
        if ty1.is_never_type() {
            true
        } else if ty1.equals_to(ty2) {
            true
        } else if let TyBody::TyParamRef { name, .. } = &ty1.body {
            if let TyBody::TyParamRef { name: name2, .. } = &ty2.body {
                name == name2
            } else {
                ty2 == &ty::raw("Object") // The upper bound
            }
        } else if let TyBody::TyParamRef { name, .. } = &ty2.body {
            if let TyBody::TyParamRef { name: name2, .. } = &ty1.body {
                name == name2
            } else {
                false
            }
        } else {
            let is_void_fn = if let Some(ret_ty) = ty2.fn_x_info() {
                ret_ty.is_void_type()
            } else {
                false
            };
            if let Some(t1) = self.ancestor_types(ty1).iter().find(|t| t.same_base(&ty2)) {
                if t1.equals_to(&ty2) {
                    return true;
                } else if t1.tyargs().iter().all(|t| t.is_never_type()) {
                    return true;
                } else {
                    // Special care for void funcs
                    return is_void_fn;
                }
            }
            false
        }
    }

    pub fn find_ivar(&self, classname: &ClassFullname, ivar_name: &str) -> Option<&SkIVar> {
        let class = self.sk_classes.get(&classname).unwrap_or_else(|| {
            panic!(
                "[BUG] ClassDict::find_ivar: class `{}' not found",
                &classname
            )
        });
        class.ivars.get(ivar_name)
    }

    /// Returns instance variables of the superclass of `classname`
    pub fn superclass_ivars(&self, classname: &ClassFullname) -> Option<SkIVars> {
        self.get_class(&classname).superclass.as_ref().map(|scls| {
            let ty = scls.ty();
            let ivars = &self.get_class(&ty.erasure()).ivars;
            let tyargs = ty.tyargs();
            ivars
                .iter()
                .map(|(name, ivar)| (name.clone(), ivar.substitute(tyargs)))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::hir::class_dict::ClassDict;
    use crate::ty;

    fn test_class_dict<F>(s: &str, f: F) -> Result<(), Error>
    where
        F: FnOnce(ClassDict) -> (),
    {
        let core = crate::runner::load_builtin_exports()?;
        let ast = crate::parser::Parser::parse(s)?;
        let class_dict =
            crate::hir::class_dict::create(&ast, Default::default(), &core.sk_classes)?;
        f(class_dict);
        Ok(())
    }

    #[test]
    fn test_supertype_default() -> Result<(), Error> {
        let src = "";
        test_class_dict(src, |class_dict| {
            assert_eq!(
                class_dict.supertype(&ty::ary(ty::raw("Int"))),
                Some(ty::raw("Object"))
            )
        })
    }

    #[test]
    fn test_supertype_gen() -> Result<(), Error> {
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
    fn test_is_descendant_simple() -> Result<(), Error> {
        let src = "class A; end";
        test_class_dict(src, |class_dict| {
            assert!(class_dict.is_descendant(&ty::raw("A"), &ty::raw("Object")));
        })
    }

    #[test]
    fn test_is_descendant_simple_inheritance() -> Result<(), Error> {
        let src = "
          class A; end
          class B : A; end
        ";
        test_class_dict(src, |class_dict| {
            assert!(class_dict.is_descendant(&ty::raw("B"), &ty::raw("A")));
            assert!(class_dict.is_descendant(&ty::raw("B"), &ty::raw("Object")));
        })
    }

    #[test]
    fn test_is_descendant_inherit_spe() -> Result<(), Error> {
        let src = "class A : Array<Int>; end";
        test_class_dict(src, |class_dict| {
            assert!(class_dict.is_descendant(&ty::raw("A"), &ty::ary(ty::raw("Int"))));
            assert!(class_dict.is_descendant(&ty::raw("A"), &ty::raw("Object")));
        })
    }

    #[test]
    fn test_is_descendant_inherit_gen() -> Result<(), Error> {
        let src = "
            class MyMaybe<T>; end
            class MySome<T> : MyMaybe<T>; end
        ";
        test_class_dict(src, |class_dict| {
            let some_int = ty::spe("MySome", vec![ty::raw("Int")]);
            let maybe_int = ty::spe("MyMaybe", vec![ty::raw("Int")]);
            assert!(class_dict.is_descendant(&some_int, &maybe_int));
        })
    }

    #[test]
    fn test_is_descendant_invariant() -> Result<(), Error> {
        let src = "class A<T>; end";
        test_class_dict(src, |class_dict| {
            let a_int = ty::spe("A", vec![ty::raw("Int")]);
            let a_obj = ty::spe("A", vec![ty::raw("Object")]);
            assert!(!class_dict.is_descendant(&a_int, &a_obj));
        })
    }

    #[test]
    fn test_conforms_some() -> Result<(), Error> {
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
    fn test_conforms_none() -> Result<(), Error> {
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
    fn test_conforms_invalid() -> Result<(), Error> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let a = ty::raw("Int");
            let b = ty::raw("Bool");
            assert!(!class_dict.conforms(&a, &b));
        })
    }

    #[test]
    fn test_conforms_not() -> Result<(), Error> {
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
    fn test_conforms_void_func() -> Result<(), Error> {
        let src = "";
        test_class_dict(src, |class_dict| {
            let a = ty::spe("Fn0", vec![ty::raw("Int")]);
            let b = ty::spe("Fn0", vec![ty::raw("Void")]);
            assert!(class_dict.conforms(&a, &b));
        })
    }
}
