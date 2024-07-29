pub mod asyncness_check;
pub mod blocked;
mod expr;
pub mod rewriter;
mod ty;
pub mod typing;
pub mod untyped;
pub mod visitor;
use crate::ast;
use anyhow::Result;
pub use expr::{yielded_ty, CastType, Expr, PseudoVar, Typed, TypedExpr};
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
    pub is_async: bool,
    pub is_internal: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Ty,
}

impl TryFrom<ast::Extern> for Extern {
    type Error = anyhow::Error;
    fn try_from(x: ast::Extern) -> Result<Self> {
        Extern::from_ast(&x)
    }
}

impl Extern {
    pub fn from_ast(x: &ast::Extern) -> Result<Self> {
        Ok(Self {
            is_async: x.is_async,
            is_internal: x.is_internal,
            name: x.name.clone(),
            params: x
                .params
                .iter()
                .map(|x| x.clone().try_into())
                .collect::<Result<_>>()?,
            ret_ty: x.ret_ty.clone().try_into()?,
        })
    }

    pub fn fun_ty(&self) -> FunTy {
        FunTy {
            asyncness: self.is_async.into(),
            param_tys: self.params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>(),
            ret_ty: Box::new(self.ret_ty.clone()),
        }
    }
}

impl fmt::Display for Extern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let asyn = if self.is_async { "(async)" } else { "" };
        let inte = if self.is_internal { "(internal)" } else { "" };
        let para = self
            .params
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "extern{}{} {}({}) -> {};\n",
            asyn, inte, self.name, para, self.ret_ty
        )
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub generated: bool,
    pub asyncness: Asyncness,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Ty,
    pub body_stmts: Vec<Typed<Expr>>,
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
        for expr in &self.body_stmts {
            write!(f, "  {}  #-> {}\n", &expr.0, &expr.1)?;
        }
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

impl TryFrom<ast::Param> for Param {
    type Error = anyhow::Error;

    fn try_from(x: ast::Param) -> Result<Self> {
        Ok(Self {
            ty: x.ty.try_into()?,
            name: x.name,
        })
    }
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
