use super::SkTypeBase;
use crate::sk_type::wtable::WTable;
use crate::supertype::Supertype;
use crate::{SkIVar, SkIVars};
use serde::{Deserialize, Serialize};
use shiika_core::names::ClassFullname;
use shiika_core::ty::{LitTy, TermTy, TyBody};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SkClass {
    pub base: SkTypeBase,
    pub superclass: Option<Supertype>,
    /// Included modules
    pub includes: Vec<Supertype>,
    pub ivars: HashMap<String, SkIVar>,
    /// true if this class is declared as base class.
    pub inheritable: bool,
    /// True if the constant of the class name holds the only instance instead
    /// of the class object. (eg. `Void`, `None`)
    pub const_is_obj: bool,
    /// Witness table
    pub wtable: WTable,
}

impl SkClass {
    pub fn nonmeta(base: SkTypeBase, superclass: Option<Supertype>) -> SkClass {
        SkClass {
            base,
            superclass,
            includes: Default::default(),
            ivars: Default::default(),
            inheritable: Default::default(),
            const_is_obj: false,
            wtable: Default::default(),
        }
    }

    pub fn meta(base: SkTypeBase) -> SkClass {
        SkClass {
            base,
            superclass: Some(Supertype::simple("Class")),
            includes: Default::default(),
            ivars: Default::default(),
            inheritable: Default::default(),
            const_is_obj: false,
            wtable: Default::default(),
        }
    }

    pub fn lit_ty(&self) -> LitTy {
        self.base.erasure.to_lit_ty()
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

    pub fn ivars_ordered(&self) -> Vec<SkIVar> {
        let mut v = self.ivars.values().cloned().collect::<Vec<_>>();
        v.sort_by_key(|x| x.idx);
        v
    }

    /// Returns supertype of `self` with given `type_args`.
    /// eg. given `class B<Y, X> : A<X>` and `self` is `B` and `type_args` is `[Int, Bool]`,
    /// returns `A<Bool>`.
    pub fn specialized_superclass(&self, type_args: &[TermTy]) -> Option<LitTy> {
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
    pub fn specialize_module(&self, modinfo: &Supertype, type_args: &[TermTy]) -> Vec<TermTy> {
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
