use crate::names::*;
use crate::ty;
use crate::ty::*;
use std::collections::HashMap;

/// A Shiika class, possibly generic
#[derive(Debug, PartialEq, Clone)]
pub struct SkClass {
    pub fullname: ClassFullname,
    pub typarams: Vec<TyParam>,
    pub superclass_fullname: Option<ClassFullname>,
    pub instance_ty: TermTy,
    pub ivars: HashMap<String, super::SkIVar>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
}

impl SkClass {
    pub fn class_ty(&self) -> TermTy {
        self.instance_ty.meta_ty()
    }

    /// List of method names, alphabetic order
    pub fn method_names(&self) -> Vec<MethodFullname> {
        let mut v = self
            .method_sigs
            .values()
            .map(|x| x.fullname.clone())
            .collect::<Vec<_>>();
        // Sort by first name
        v.sort_unstable_by(|a, b| a.first_name.0.cmp(&b.first_name.0));
        v
    }

    /// Create a specialized metaclass of a generic metaclass
    /// eg. create `Meta:Array<Int>` from `Meta:Array`
    pub fn specialized_meta(&self, tyargs: &[TermTy]) -> SkClass {
        // `self` must be a generic metaclass.
        debug_assert!(self.typarams.len() > 0);
        let base_name = if let TyBody::TyMeta { base_fullname } = &self.instance_ty.body {
            base_fullname
        } else {
            panic!("SkClass::specialize: not TyMeta")
        };
        let instance_ty = ty::spe_meta(&base_name, tyargs.to_vec());
        let method_sigs = self
            .method_sigs
            .iter()
            .map(|(name, sig)| (name.clone(), sig.specialize(tyargs)))
            .collect(); //::<Vec<_>>;

        SkClass {
            fullname: instance_ty.fullname.clone(),
            typarams: vec![],
            superclass_fullname: self.superclass_fullname.clone(),
            instance_ty,
            ivars: self.ivars.clone(),
            method_sigs,
            const_is_obj: self.const_is_obj,
        }
    }
}
