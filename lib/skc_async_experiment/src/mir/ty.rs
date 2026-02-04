use crate::mir::Asyncness;
use anyhow::Context;
use shiika_core::ty::TermTy;
use skc_hir::MethodSignature;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Ptr,   // Corresponds to `ptr` in llvm
    Any,   // Opaque value converted to `i64` (to store it in ChiikaEnv)
    I1,    // Corresponds to `i1` in llvm
    Int64, // Corresponds to `i64` in llvm
    CVoid, // Corresponds to `void` in llvm
    ChiikaEnv,
    RustFuture,
    Sk(TermTy), // A Shiika value
    Fun(FunTy), // C-level(=llvm-level) function type
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
        match ty.fn_x_info() {
            Some(tys) => {
                let mut param_tys = tys
                    .into_iter()
                    .map(|x| x.clone().into())
                    .collect::<Vec<_>>();
                let ret_ty = param_tys.pop().unwrap();
                Ty::Fun(FunTy {
                    asyncness: Asyncness::Unknown,
                    param_tys,
                    ret_ty: Box::new(ret_ty),
                })
            }
            None => match &ty.fullname.0[..] {
                "Shiika::Internal::Ptr" => Ty::Ptr,
                "Shiika::Internal::Int64" => Ty::Int64,
                _ => Ty::Sk(ty),
            },
        }
    }
}

impl Ty {
    pub fn raw(s: impl Into<String>) -> Self {
        Ty::Sk(shiika_core::ty::raw(s))
    }

    pub fn meta(s: impl Into<String>) -> Self {
        Ty::Sk(shiika_core::ty::meta(s))
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
            Ty::Fun(f) => Some(f.is_async()),
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
            Ty::Ptr => 0,
            Ty::Any => 1,
            Ty::I1 => 2,
            Ty::Int64 => 3,
            Ty::CVoid => panic!("CVoid has no value"),
            Ty::ChiikaEnv => 4,
            Ty::RustFuture => 5,
            Ty::Sk(_) => 6,
            Ty::Fun(_) => 7,
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

    pub fn from_sig(sig: MethodSignature) -> Self {
        let param_tys = sig.param_tys().into_iter().map(|x| x.into()).collect();
        Self::new(sig.asyncness.into(), param_tys, sig.ret_ty.into())
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

    pub fn is_async(&self) -> bool {
        self.asyncness
            .is_async()
            .context(format!("{:?}", self))
            .unwrap()
    }
}
