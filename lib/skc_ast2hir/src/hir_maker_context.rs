use shiika_core::{names::*, ty::*};
use skc_hir::{MethodParam, MethodSignature, SkIVars};
use std::collections::HashMap;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
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

    pub fn class(namespace: Namespace, typarams: Vec<TyParam>) -> HirMakerContext {
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
    pub typarams: Vec<TyParam>,
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

impl LambdaCtx {
    /// Push a LambdaCapture to captures
    pub fn push_lambda_capture(&mut self, cap: LambdaCapture) -> usize {
        self.captures.push(cap);
        self.captures.len() - 1
    }

    pub fn update_capture_ty(&mut self, cidx: usize, ty: TermTy) {
        let cap = &mut self.captures[cidx];
        cap.ty = ty;
        cap.upcast_needed = true;
    }

    /// Returns cidx if `cap` is already in the `captuers`.
    pub fn check_already_captured(&self, cap: &LambdaCapture) -> Option<usize> {
        self.captures.iter().position(|x| x.equals(cap))
    }
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
    /// The index of ctx stack where this lvar is captured.
    /// None if the lvar does not belong to a lambda (method argument, etc.)
    pub ctx_depth: Option<usize>,
    pub ty: TermTy,
    pub upcast_needed: bool,
    pub detail: LambdaCaptureDetail,
}

#[derive(Debug)]
pub enum LambdaCaptureDetail {
    CapLVar { name: String },
    CapFnArg { idx: usize },
}

impl LambdaCapture {
    fn equals(&self, other: &LambdaCapture) -> bool {
        if self.ctx_depth != other.ctx_depth {
            return false;
        }
        let equals = match (&self.detail, &other.detail) {
            (
                LambdaCaptureDetail::CapLVar { name },
                LambdaCaptureDetail::CapLVar { name: name2 },
            ) => name == name2,
            (
                LambdaCaptureDetail::CapFnArg { idx },
                LambdaCaptureDetail::CapFnArg { idx: idx2 },
            ) => idx == idx2,
            _ => false,
        };
        if equals {
            debug_assert!(self.ty == other.ty);
        }
        equals
    }
}
