use crate::hir::Asyncness;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Unknown, // Used before typecheck
    Raw(String),
    Fun(FunTy),
}

impl Ty {
    pub fn raw(s: impl Into<String>) -> Self {
        Ty::Raw(s.into())
    }

    pub fn as_fun_ty(&self) -> &FunTy {
        match self {
            Ty::Fun(f) => f,
            _ => panic!("[BUG] not a function type: {:?}", self),
        }
    }

    pub fn into_fun_ty(self) -> FunTy {
        match self {
            Ty::Fun(f) => f,
            _ => panic!("[BUG] not a function type: {:?}", self),
        }
    }

    /// Returns Some(true) if the type is a function type and it is async.
    /// Returns Some(false) if the type is a function type and it is sync.
    /// Returns None if the type is not a function type.
    pub fn is_async_fun(&self) -> Option<bool> {
        match self {
            Ty::Fun(f) => Some(f.asyncness.is_async()),
            _ => None,
        }
    }

    /// Returns true if the two function types are the same except for asyncness.
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Ty::Fun(f1), Ty::Fun(f2)) => f1.same(f2),
            _ => self == other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunTy {
    pub asyncness: Asyncness,
    pub param_tys: Vec<Ty>,
    pub ret_ty: Box<Ty>,
}

impl From<FunTy> for Ty {
    fn from(x: FunTy) -> Self {
        Ty::Fun(x)
    }
}

impl From<Ty> for FunTy {
    fn from(x: Ty) -> Self {
        match x {
            Ty::Fun(f) => f,
            _ => panic!("[BUG] not a function type: {:?}", x),
        }
    }
}

impl FunTy {
    fn new(asyncness: Asyncness, param_tys: Vec<Ty>, ret_ty: Ty) -> Self {
        FunTy {
            asyncness,
            param_tys,
            ret_ty: Box::new(ret_ty),
        }
    }

    pub fn sync(param_tys: Vec<Ty>, ret_ty: Ty) -> Self {
        Self::new(Asyncness::Sync, param_tys, ret_ty)
    }

    pub fn async_(param_tys: Vec<Ty>, ret_ty: Ty) -> Self {
        Self::new(Asyncness::Async, param_tys, ret_ty)
    }

    pub fn lowered(param_tys: Vec<Ty>, ret_ty: Ty) -> Self {
        Self::new(Asyncness::Lowered, param_tys, ret_ty)
    }

    /// Returns true if the two function types are the same except for asyncness.
    pub fn same(&self, other: &Self) -> bool {
        self.ret_ty.same(&other.ret_ty)
            && self.param_tys.len() == other.param_tys.len()
            && self
                .param_tys
                .iter()
                .zip(other.param_tys.iter())
                .all(|(a, b)| a.same(b))
    }
}
