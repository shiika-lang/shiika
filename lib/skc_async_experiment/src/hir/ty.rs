use crate::hir::Asyncness;
use shiika_core::ty::{self, TermTy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunTy {
    pub asyncness: Asyncness,
    pub param_tys: Vec<TermTy>,
    pub ret_ty: TermTy,
}

impl FunTy {
    fn new(asyncness: Asyncness, param_tys: Vec<TermTy>, ret_ty: TermTy) -> Self {
        FunTy {
            asyncness,
            param_tys,
            ret_ty,
        }
    }

    pub fn sync(param_tys: Vec<TermTy>, ret_ty: TermTy) -> Self {
        Self::new(Asyncness::Sync, param_tys, ret_ty)
    }

    pub fn async_(param_tys: Vec<TermTy>, ret_ty: TermTy) -> Self {
        Self::new(Asyncness::Async, param_tys, ret_ty)
    }

    pub fn lowered(param_tys: Vec<TermTy>, ret_ty: TermTy) -> Self {
        Self::new(Asyncness::Lowered, param_tys, ret_ty)
    }

    pub fn to_term_ty(self) -> TermTy {
        let base_name = format!("Fn{}", self.param_tys.len());
        let mut ts = self.param_tys;
        ts.push(self.ret_ty);
        ty::nonmeta(base_name, ts)
    }
}
