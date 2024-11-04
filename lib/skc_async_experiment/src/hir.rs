pub mod asyncness_check;
pub mod expr;
pub mod rewriter;
mod ty;
pub mod typing;
pub mod untyped;
pub mod visitor;
use crate::names::FunctionName;
pub use expr::{CastType, Expr, PseudoVar, Typed, TypedExpr};
use std::fmt;
pub use ty::{FunTy, Ty};

#[derive(Debug, Clone)]
pub struct Program {
    pub externs: Vec<Extern>,
    pub funcs: Vec<Function>,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.externs {
            write!(f, "{}", e)?;
        }
        for func in &self.funcs {
            write!(f, "{}", func)?;
        }
        write!(f, "")
    }
}

impl Program {
    pub fn new(externs: Vec<Extern>, funcs: Vec<Function>) -> Self {
        Self { externs, funcs }
    }
}

#[derive(Debug, Clone)]
pub struct Extern {
    pub name: FunctionName,
    pub fun_ty: FunTy,
}

impl fmt::Display for Extern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "extern({}) {} {};\n",
            self.fun_ty.asyncness, self.name, self.fun_ty
        )
    }
}

impl Extern {
    pub fn is_async(&self) -> bool {
        self.fun_ty.asyncness.is_async()
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub generated: bool,
    pub asyncness: Asyncness,
    pub name: FunctionName,
    pub params: Vec<Param>,
    pub ret_ty: Ty,
    pub body_stmts: Typed<Expr>,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let gen = if self.generated { "." } else { "" };
        let para = self
            .params
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "fun{} {}{}({}) -> {} {{\n",
            gen, self.name, self.asyncness, para, self.ret_ty
        )?;
        write!(f, "{}\n", &self.body_stmts.0.pretty_print(1, true),)?;
        write!(f, "}}\n")
    }
}

impl Function {
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

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.ty, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asyncness {
    Unknown,
    Sync,
    Async,
    Lowered,
}

impl fmt::Display for Asyncness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Asyncness::Unknown => write!(f, "[?]"),
            Asyncness::Sync => write!(f, "[+]"),
            Asyncness::Async => write!(f, "[*]"),
            Asyncness::Lowered => write!(f, ""), // "[.]"
        }
    }
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
