use crate::mir::Asyncness;
use shiika_core::ty::TermTy;
use skc_hir::MethodSignature;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Any,   // Corresponds to `ptr` in llvm
    I1,    // Corresponds to `i1` in llvm
    Int64, // Corresponds to `i64` in llvm
    ChiikaEnv,
    RustFuture,
    Raw(String),
    Fun(FunTy),
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Fun(fun_ty) => write!(f, "{}", fun_ty),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl From<TermTy> for Ty {
    fn from(ty: TermTy) -> Self {
        // TODO: typaram ref
        Ty::Raw(ty.fullname.0)
    }
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

    pub fn type_id(&self) -> i64 {
        match self {
            Ty::Raw(_) => 0,
            Ty::Any => 1,
            Ty::ChiikaEnv => 2,
            Ty::RustFuture => 3,
            Ty::Fun(_) => 4,
            _ => panic!("[BUG] unknown type: {:?}", self),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FunTy {
    pub asyncness: Asyncness,
    pub param_tys: Vec<Ty>,
    pub ret_ty: Box<Ty>,
}

impl fmt::Display for FunTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let para = self
            .param_tys
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{}({})->{}", &self.asyncness, para, &self.ret_ty)
    }
}

impl fmt::Debug for FunTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
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
    pub fn new(asyncness: Asyncness, param_tys: Vec<Ty>, ret_ty: Ty) -> Self {
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

    /// Creates a FunTy of a Shiika method compiled into LLVM func.
    pub fn from_method_signature(sig: MethodSignature) -> Self {
        let receiver_ty = sig.receiver_ty().into();
        let mut param_tys = sig
            .params
            .into_iter()
            .map(|p| p.ty.into())
            .collect::<Vec<_>>();
        param_tys.insert(0, receiver_ty);
        Self::new(sig.asyncness.into(), param_tys, sig.ret_ty.into())
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
