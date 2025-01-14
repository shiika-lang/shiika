pub mod expr;
mod ty;
pub mod typing;
pub mod untyped;
use crate::names::FunctionName;
pub use expr::{Expr, TypedExpr};
use shiika_core::ty::TermTy;
pub use ty::FunTy;

#[derive(Debug, Clone)]
pub struct Program<T> {
    pub externs: Vec<Extern>,
    pub methods: Vec<Method<T>>,
}

impl<T> Program<T> {
    pub fn new(externs: Vec<Extern>, methods: Vec<Method<T>>) -> Self {
        Self { externs, methods }
    }
}

#[derive(Debug, Clone)]
pub struct Extern {
    pub name: FunctionName,
    pub fun_ty: FunTy,
}

impl Extern {
    pub fn is_async(&self) -> bool {
        self.fun_ty.asyncness.is_async()
    }
}

#[derive(Debug, Clone)]
pub struct Method<T> {
    pub asyncness: Asyncness,
    pub name: FunctionName,
    pub params: Vec<Param>,
    pub ret_ty: TermTy,
    pub body_stmts: TypedExpr<T>,
}

impl<T: Clone> Method<T> {
    pub fn fun_ty(&self) -> FunTy {
        FunTy {
            asyncness: self.asyncness.clone(),
            param_tys: self.params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>(),
            ret_ty: self.ret_ty.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ty: TermTy,
    pub name: String,
}

impl Param {
    pub fn new(ty: TermTy, name: impl Into<String>) -> Self {
        Self {
            ty,
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asyncness {
    Unknown,
    Sync,
    Async,
    Lowered,
}

impl From<bool> for Asyncness {
    fn from(x: bool) -> Self {
        if x {
            Asyncness::Async
        } else {
            Asyncness::Sync
        }
    }
}

impl Asyncness {
    /// Returns true for Asyncness::Async. Panics if not applicable
    pub fn is_async(&self) -> bool {
        match self {
            Asyncness::Unknown => panic!("[BUG] asyncness is unknown"),
            Asyncness::Async => true,
            Asyncness::Sync => false,
            Asyncness::Lowered => panic!("[BUG] asyncness is lost"),
        }
    }

    pub fn is_sync(&self) -> bool {
        !self.is_async()
    }
}
