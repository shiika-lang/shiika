use crate::names::module_fullname;
use crate::ty::lit_ty::LitTy;
use crate::ty::term_ty::{TermTy, TyBody};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TyParamRef {
    pub kind: TyParamKind,
    pub name: String,
    pub idx: usize,
    pub upper_bound: LitTy,
    pub lower_bound: LitTy,
    /// Whether referring this typaram as a class object (eg. `p T`)
    pub as_class: bool,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum TyParamKind {
    /// eg. `class A<B>`
    Class,
    /// eg. `def foo<X>(...)`
    Method,
}

impl TyParamRef {
    pub fn dbg_str(&self) -> String {
        let k = match &self.kind {
            TyParamKind::Class => "C",
            TyParamKind::Method => "M",
        };
        let c = if self.as_class { "!" } else { " " };
        format!("TyParamRef({}{}{}{})", &self.name, c, &self.idx, k)
    }

    pub fn to_term_ty(&self) -> TermTy {
        self.clone().into_term_ty()
    }

    pub fn into_term_ty(self) -> TermTy {
        TermTy {
            // TODO: self.name (eg. "T") is not a class name. Should remove fullname from TermTy?
            fullname: module_fullname(&self.name),
            body: TyBody::TyPara(self),
        }
    }

    pub fn as_class(&self) -> TyParamRef {
        debug_assert!(!self.as_class);
        let mut ref2 = self.clone();
        ref2.as_class = true;
        ref2
    }

    pub fn as_type(&self) -> TyParamRef {
        debug_assert!(self.as_class);
        let mut ref2 = self.clone();
        ref2.as_class = false;
        ref2
    }
}
