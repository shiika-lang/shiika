use crate::error;
use crate::error::*;
use crate::hir::class_dict::class_dict::ClassDict;
use crate::hir::*;
use crate::names::*;
use crate::ty::*;

impl ClassDict {
    /// Find a method from class name and first name
    pub fn find_method(
        &self,
        class_fullname: &ClassFullname,
        method_name: &MethodFirstname,
    ) -> Option<&MethodSignature> {
        self.sk_classes
            .get(class_fullname)
            .and_then(|class| class.method_sigs.get(method_name))
    }

    /// Similar to find_method, but lookup into superclass if not in the class.
    /// Returns Err if not found.
    pub fn lookup_method(
        &self,
        class: &TermTy,
        method_name: &MethodFirstname,
        method_tyargs: &[TermTy],
    ) -> Result<(MethodSignature, ClassFullname), Error> {
        match &class.body {
            TyBody::TyRaw | TyBody::TyMeta { .. } | TyBody::TyClass => {
                self.lookup_method_(class, class, method_name)
            }
            TyBody::TySpe { type_args, .. } | TyBody::TySpeMeta { type_args, .. } => {
                let base_cls = &self
                    .find_class(&class.base_class_name())
                    .expect("[BUG] base_cls not found")
                    .instance_ty;
                let (base_sig, found_cls) = self.lookup_method_(base_cls, base_cls, method_name)?;
                Ok((base_sig.specialize(&type_args, method_tyargs), found_cls))
            }
            TyBody::TyParamRef { .. } => {
                let o = ty::raw("Object");
                self.lookup_method_(&o, &o, method_name)
            }
            _ => todo!("{}", class),
        }
    }

    fn lookup_method_(
        &self,
        receiver_class: &TermTy,
        class: &TermTy,
        method_name: &MethodFirstname,
    ) -> Result<(MethodSignature, ClassFullname), Error> {
        if let Some(sig) = self.find_method(&class.fullname, method_name) {
            Ok((sig.clone(), class.fullname.clone()))
        } else {
            // Look up in superclass
            let sk_class = self.find_class(&class.fullname).unwrap_or_else(|| {
                panic!(
                    "[BUG] lookup_method: asked to find `{}' but class `{}' not found",
                    &method_name.0, &class.fullname.0
                )
            });
            if let Some(super_name) = &sk_class.superclass_fullname {
                // TODO #115: super may not be a ty::raw
                let super_class = ty::raw(&super_name.0);
                self.lookup_method_(receiver_class, &super_class, method_name)
            } else {
                Err(error::program_error(&format!(
                    "method {:?} not found on {:?}",
                    method_name, receiver_class.fullname
                )))
            }
        }
    }

    /// Find a class
    pub fn find_class(&self, class_fullname: &ClassFullname) -> Option<&SkClass> {
        self.sk_classes.get(class_fullname)
    }

    /// Find a class. Panic if not found
    pub fn get_class(&self, class_fullname: &ClassFullname, dbg_name: &str) -> &SkClass {
        self.find_class(class_fullname).unwrap_or_else(|| {
            panic!(
                "[BUG] {}: class `{}' not found",
                &dbg_name, &class_fullname.0
            )
        })
    }

    /// Find a class. Panic if not found
    pub fn get_class_mut(
        &mut self,
        class_fullname: &ClassFullname,
        dbg_name: &str,
    ) -> &mut SkClass {
        self.sk_classes.get_mut(&class_fullname).unwrap_or_else(|| {
            panic!(
                "[BUG] {}: class `{}' not found",
                &dbg_name, &class_fullname.0
            )
        })
    }

    /// Return true if there is a class of the name
    pub fn class_exists(&self, class_fullname: &str) -> bool {
        self.sk_classes
            .contains_key(&ClassFullname(class_fullname.to_string()))
    }

    /// Find the superclass
    /// Return None if the class is `Object`
    pub fn get_superclass(&self, classname: &ClassFullname) -> Option<&SkClass> {
        let cls = self.get_class(&classname, "ClassDict::get_superclass");
        cls.superclass_fullname
            .as_ref()
            .map(|super_name| self.get_class(&super_name, "ClassDict::get_superclass"))
    }

    /// Return supertype of `ty`
    pub fn supertype_of(&self, ty: &TermTy) -> Option<TermTy> {
        ty.supertype(self)
    }

    /// Return ancestor types of `ty`, including itself.
    pub fn ancestor_types(&self, ty: &TermTy) -> Vec<TermTy> {
        let mut v = vec![];
        let mut t = Some(ty.clone());
        while t.is_some() {
            v.push(t.unwrap());
            t = self.supertype_of(&v.last().unwrap())
        }
        v
    }

    /// Return true if ty1 is an descendant of ty2
    pub fn is_descendant(&self, ty1: &TermTy, ty2: &TermTy) -> bool {
        let expected = Some(ty2.clone());
        let mut t = Some(ty1.clone());
        while t.is_some() {
            t = self.supertype_of(&t.unwrap());
            if t == expected {
                return true;
            }
        }
        false
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
}
