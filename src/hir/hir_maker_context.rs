use crate::hir::*;
use crate::names::*;
use crate::ty::*;
use std::collections::HashMap;

#[derive(Debug)]
pub enum HirMakerContext {
    Toplevel(ToplevelCtx),
    Class(ClassCtx),
    Method(MethodCtx),
    Lambda(LambdaCtx),
    While(WhileCtx),
    MatchClause(MatchClauseCtx),
}

impl HirMakerContext {
    /// Get the hashmap of local variables
    pub fn opt_lvars(&mut self) -> Option<&mut CtxLVars> {
        match self {
            HirMakerContext::Toplevel(c) => Some(&mut c.lvars),
            HirMakerContext::Class(c) => Some(&mut c.lvars),
            HirMakerContext::Method(c) => Some(&mut c.lvars),
            HirMakerContext::Lambda(c) => Some(&mut c.lvars),
            HirMakerContext::MatchClause(c) => Some(&mut c.lvars),
            HirMakerContext::While(_) => None,
        }
    }

    pub fn toplevel() -> HirMakerContext {
        HirMakerContext::Toplevel(ToplevelCtx {
            lvars: Default::default(),
        })
    }

    pub fn class(namespace: Namespace, typarams: Vec<String>) -> HirMakerContext {
        HirMakerContext::Class(ClassCtx {
            namespace,
            typarams,
            lvars: Default::default(),
        })
    }

    pub fn method(signature: MethodSignature, super_ivars: Option<SkIVars>) -> HirMakerContext {
        HirMakerContext::Method(MethodCtx {
            signature,
            lvars: Default::default(),
            iivars: Default::default(),
            super_ivars: super_ivars.unwrap_or_default(),
        })
    }

    pub fn lambda(is_fn: bool, params: Vec<MethodParam>) -> HirMakerContext {
        HirMakerContext::Lambda(LambdaCtx {
            is_fn,
            params,
            lvars: Default::default(),
            captures: Default::default(),
            has_break: false,
        })
    }

    // `while' is Rust's keyword
    pub fn while_ctx() -> HirMakerContext {
        HirMakerContext::While(WhileCtx {})
    }

    pub fn match_clause() -> HirMakerContext {
        HirMakerContext::MatchClause(MatchClauseCtx {
            lvars: Default::default(),
        })
    }
}

#[derive(Debug)]
pub struct ToplevelCtx {
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
}

#[derive(Debug)]
pub struct ClassCtx {
    /// Current namespace
    pub namespace: Namespace,
    /// Names of class type parameters
    pub typarams: Vec<String>,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
}

#[derive(Debug)]
pub struct MethodCtx {
    /// Signature of the current method
    pub signature: MethodSignature,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
    /// List of instance variables in an initializer found so far.
    /// Empty if the method is not `#initialize`
    pub iivars: SkIVars,
    /// List of inherited ivars
    /// Empty if the method is not `#initialize`
    pub super_ivars: SkIVars, // TODO: this can be just &'a SkIVars
}

#[derive(Debug)]
pub struct LambdaCtx {
    /// true if this lambda is `fn(){}`. false if it is a block (`do..end`,`{...}`)
    pub is_fn: bool,
    /// Parameters of the lambda
    pub params: Vec<MethodParam>,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
    /// List of free variables captured in this context
    pub captures: Vec<LambdaCapture>,
    /// true if this lambda has `break`
    pub has_break: bool,
}

/// Indicates we're in a while expr
#[derive(Debug)]
pub struct WhileCtx;

/// Each clause of match expression has its own lvars
#[derive(Debug)]
pub struct MatchClauseCtx {
    /// Local variables introduced when matched
    pub lvars: HashMap<String, CtxLVar>,
}

/// A local variable
#[derive(Debug)]
pub struct CtxLVar {
    pub name: String,
    pub ty: TermTy,
    pub readonly: bool,
}

pub type CtxLVars = HashMap<String, CtxLVar>;

#[derive(Debug)]
pub struct LambdaCapture {
    /// The index of `self.ctx.lambdas` where this lvar is captured.
    /// -1 if it is captured in `self.ctx.method` or `self.ctx.toplevel`
    pub ctx_depth: isize,
    pub ty: TermTy,
    pub detail: LambdaCaptureDetail,
}

#[derive(Debug)]
pub enum LambdaCaptureDetail {
    CapLVar { name: String },
    CapFnArg { idx: usize },
}
