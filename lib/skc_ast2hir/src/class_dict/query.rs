use crate::class_dict::*;
use crate::error;
use anyhow::Result;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::*;

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
    /// Returns the class where the method is found as a `TermTy`.
    /// Returns Err if not found.
    pub fn lookup_method(
        &self,
        receiver_class: &TermTy,
        method_name: &MethodFirstname,
        method_tyargs: &[TermTy],
    ) -> Result<(MethodSignature, TermTy)> {
        self.lookup_method_(receiver_class, receiver_class, method_name, method_tyargs)
    }

    fn lookup_method_(
        &self,
        receiver_class: &TermTy,
        class: &TermTy,
        method_name: &MethodFirstname,
        method_tyargs: &[TermTy],
    ) -> Result<(MethodSignature, TermTy)> {
        let ty_obj = ty::raw("Object");
        let (class, class_tyargs) = match &class.body {
            TyBody::TyRaw(LitTy { type_args, .. }) => {
                let base_cls = &self.get_class(&class.base_class_name()).instance_ty;
                (base_cls, type_args.as_slice())
            }
            TyBody::TyParamRef { .. } => (&ty_obj, Default::default()),
        };
        if let Some(sig) = self.find_method(&class.fullname, method_name) {
            Ok((sig.specialize(class_tyargs, method_tyargs), class.clone()))
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
        match &ty.body {
            TyBody::TyParamRef { upper_bound, .. } => Some(*upper_bound.clone()),
            _ => self
                .get_class(&ty.erasure())
                .superclass
                .as_ref()
                .map(|scls| scls.ty().substitute(ty.tyargs(), &[])),
        }
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

    /// Returns the nearest common ancestor of the classes
    /// Returns `None` if there is no common ancestor except `Object`, the
    /// top type. However, returns `Some(Object)` when either of the arguments
    /// is `Object`.
    pub fn nearest_common_ancestor(&self, ty1: &TermTy, ty2: &TermTy) -> Option<TermTy> {
        let t = self._nearest_common_ancestor(ty1, ty2);
        let obj = ty::raw("Object");
        if t == obj {
            if *ty1 == obj || *ty2 == obj {
                Some(obj)
            } else {
                // No common ancestor found (except `Object`)
                None
            }
        } else {
            Some(t)
        }
    }

    /// Find common ancestor of two types
    fn _nearest_common_ancestor(&self, ty1: &TermTy, ty2: &TermTy) -> TermTy {
        let ancestors1 = self.ancestor_types(ty1);
        let ancestors2 = self.ancestor_types(ty2);
        for t2 in &ancestors2 {
            let mut t = None;
            for t1 in &ancestors1 {
                if t1.equals_to(t2) {
                    t = Some(t1);
                    break;
                } else if t1.same_base(t2) {
                    if self.conforms(t1, t2) {
                        t = Some(t2);
                        break;
                    } else if self.conforms(t2, t1) {
                        t = Some(t1);
                        break;
                    }
                }
            }
            if let Some(t3) = t {
                return t3.clone();
            }
        }
        panic!("[BUG] _nearest_common_ancestor not found");
    }

    /// Return true if `ty1` conforms to `ty2` i.e.
    /// an object of the type `ty1` is included in the set of objects represented by the type `ty2`
    #[allow(clippy::if_same_then_else)]
    pub fn conforms(&self, ty1: &TermTy, ty2: &TermTy) -> bool {
        // `Never` is bottom type (i.e. subclass of any class)
        if ty1.is_never_type() {
            true
        } else if ty1.equals_to(ty2) {
            true
        } else if let TyBody::TyParamRef {
            upper_bound: u1,
            lower_bound: l1,
            ..
        } = &ty1.body
        {
            if let TyBody::TyParamRef {
                upper_bound: u2,
                lower_bound: l2,
                ..
            } = &ty2.body
            {
                u1 == u2 && l1 == l2
            } else {
                self.conforms(u1, ty2)
            }
        } else if let TyBody::TyParamRef {
            upper_bound: u2,
            lower_bound: l2,
            ..
        } = &ty2.body
        {
            if let TyBody::TyParamRef {
                upper_bound: u1,
                lower_bound: l1,
                ..
            } = &ty1.body
            {
                u1 == u2 && l1 == l2
            } else {
                self.conforms(ty1, u2)
            }
        } else {
            let is_void_fn = if let Some(ret_ty) = ty2.fn_x_info() {
                ret_ty.is_void_type()
            } else {
                false
            };
            if let Some(t1) = self.ancestor_types(ty1).iter().find(|t| t.same_base(ty2)) {
                if t1.equals_to(ty2) {
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
        let class = self.sk_classes.get(classname).unwrap_or_else(|| {
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

    fn test_class_dict<F>(s: &str, f: F) -> Result<()>
    where
        F: FnOnce(ClassDict),
    {
        let core = crate::runner::load_builtin_exports()?;
        let ast = crate::parser::Parser::parse(s)?;
        let class_dict =
            crate::hir::class_dict::create(&ast, Default::default(), &core.sk_classes)?;
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
