use crate::hir::signature::MethodSignature;
use crate::hir::superclass::Superclass;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Shiika class, possibly generic
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkClass {
    pub fullname: ClassFullname,
    pub typarams: Vec<TyParam>,
    pub superclass: Option<Superclass>,
    pub instance_ty: TermTy,
    pub ivars: HashMap<String, super::SkIVar>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
    /// true if this class cannot be a explicit superclass.
    /// None if not applicable (eg. metaclasses cannot be a explicit superclass because there is no
    /// such syntax)
    pub is_final: Option<bool>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
    /// true if this class is an imported one
    pub foreign: bool,
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
        debug_assert!(self.typarams.len() == tyargs.len());
        let base_name = if let TyBody::TyMeta { base_fullname } = &self.instance_ty.body {
            base_fullname
        } else {
            panic!("SkClass::specialize: not TyMeta: {:?}", &self.fullname)
        };
        let instance_ty = ty::spe_meta(base_name, tyargs.to_vec());
        let method_sigs = self
            .method_sigs
            .iter()
            .map(|(name, sig)| (name.clone(), sig.specialize(tyargs, Default::default())))
            .collect();
        SkClass {
            fullname: instance_ty.fullname.clone(),
            typarams: vec![],
            superclass: self.superclass.as_ref().map(|sc| sc.substitute(tyargs)),
            instance_ty,
            ivars: self.ivars.clone(),
            method_sigs,
            is_final: None,
            const_is_obj: self.const_is_obj,
            foreign: false,
        }
    }
}
