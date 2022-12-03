use super::SkTypeBase;
use crate::sk_type::wtable::WTable;
use crate::superclass::Superclass;
use crate::{SkIVar, SkIVars};
use serde::{Deserialize, Serialize};
use shiika_core::names::ClassFullname;
use shiika_core::ty::{TermTy, TyBody};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkClass {
    pub base: SkTypeBase,
    pub superclass: Option<Superclass>,
    /// Included modules (TODO: Rename `Superclass` to something better)
    pub includes: Vec<Superclass>,
    pub ivars: HashMap<String, SkIVar>,
    /// true if this class cannot be a explicit superclass.
    /// None if not applicable (eg. metaclasses cannot be a explicit superclass because there is no
    /// such syntax)
    pub is_final: Option<bool>,
    /// eg. `Void` is an instance, not the class
    pub const_is_obj: bool,
    /// Witness table
    pub wtable: WTable,
}

impl SkClass {
    pub fn nonmeta(base: SkTypeBase, superclass: Option<Superclass>) -> SkClass {
        SkClass {
            base,
            superclass,
            includes: Default::default(),
            ivars: Default::default(),
            is_final: Some(false),
            const_is_obj: false,
            wtable: Default::default(),
        }
    }

    pub fn meta(base: SkTypeBase) -> SkClass {
        SkClass {
            base,
            superclass: Some(Superclass::simple("Class")),
            includes: Default::default(),
            ivars: Default::default(),
            is_final: Some(false),
            const_is_obj: false,
            wtable: Default::default(),
        }
    }

    pub fn ivars(mut self, x: SkIVars) -> Self {
        self.ivars = x;
        self
    }

    pub fn const_is_obj(mut self, x: bool) -> Self {
        self.const_is_obj = x;
        self
    }

    pub fn fullname(&self) -> ClassFullname {
        self.base.erasure.to_class_fullname()
    }

    /// Returns supertype of `self` with given `type_args`.
    /// eg. given `class B<Y, X> : A<X>` and `self` is `B` and `type_args` is `[Int, Bool]`,
    /// returns `A<Bool>`.
    pub fn specialized_superclass(&self, type_args: &[TermTy]) -> Option<TermTy> {
        self.superclass.as_ref().map(|sup| {
            let tyargs = sup
                .type_args()
                .iter()
                .map(|t| match &t.body {
                    TyBody::TyRaw(x) => x.to_term_ty(),
                    TyBody::TyPara(tpref) => {
                        match self
                            .base
                            .typarams
                            .iter()
                            .position(|tp| tp.name == tpref.name)
                        {
                            Some(idx) => type_args[idx].clone(),
                            _ => panic!("broken superclass."),
                        }
                    }
                })
                .collect::<Vec<_>>();
            sup.ty().substitute(&tyargs, Default::default())
        })
    }

    /// Returns type args to specialize included module.
    /// eg. given `class B<Y, X> : M<X>` and `self` is `B` and `type_args` is `[Int, Bool]`,
    /// returns `[Bool]`.
    pub fn specialize_module(&self, modinfo: &Superclass, type_args: &[TermTy]) -> Vec<TermTy> {
        modinfo
            .type_args()
            .iter()
            .map(|t| match &t.body {
                TyBody::TyRaw(x) => x.to_term_ty(),
                TyBody::TyPara(tpref) => {
                    match self
                        .base
                        .typarams
                        .iter()
                        .position(|tp| tp.name == tpref.name)
                    {
                        Some(idx) => type_args[idx].clone(),
                        _ => panic!("broken superclass."),
                    }
                }
            })
            .collect::<Vec<_>>()
    }
}
