pub mod expr;
mod ty;
pub mod typing;
pub mod untyped;
use crate::names::FunctionName;
pub use expr::{Expr, Typed, TypedExpr};
pub use ty::{FunTy, Ty};

#[derive(Debug, Clone)]
pub struct Program {
    pub externs: Vec<Extern>,
    pub methods: Vec<Method>,
}

impl Program {
    pub fn new(externs: Vec<Extern>, methods: Vec<Method>) -> Self {
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
pub struct Method {
    pub asyncness: Asyncness,
    pub name: FunctionName,
    pub params: Vec<Param>,
    pub ret_ty: Ty,
    pub body_stmts: Typed<Expr>,
}

impl Method {
    pub fn fun_ty(&self) -> FunTy {
        FunTy {
            asyncness: self.asyncness.clone(),
            param_tys: self.params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>(),
            ret_ty: Box::new(self.ret_ty.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ty: Ty,
    pub name: String,
}

impl Param {
    pub fn new(ty: Ty, name: impl Into<String>) -> Self {
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
