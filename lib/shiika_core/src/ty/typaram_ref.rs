use crate::ty::term_ty::{TermTy, TyBody};
use crate::names::class_fullname;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TyParamRef {
    pub kind: TyParamKind,
    pub name: String,
    pub idx: usize,
    pub upper_bound: Box<TermTy>,
    pub lower_bound: Box<TermTy>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum TyParamKind {
    /// eg. `class A<B>`
    Class,
    /// eg. `def foo<X>(...)`
    Method,
}

impl TyParamRef {
    pub fn into_term_ty(self) -> TermTy {
        TermTy {
            // TODO: self.name (eg. "T") is not a class name. Should remove fullname from TermTy?
            fullname: class_fullname(&self.name),
            body: TyBody::TyPara(self)
        }
    }

}
