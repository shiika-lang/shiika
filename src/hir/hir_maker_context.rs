use crate::hir::hir_maker::HirMaker;
use crate::hir::*;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct HirMakerContext {
    /// Which kind of scope we're in
    pub current: CtxKind,
    // Context of each scope
    pub toplevel: ToplevelCtx,
    pub classes: Vec<ClassCtx>,
    pub method: Option<MethodCtx>,
    pub lambdas: Vec<LambdaCtx>,
}

#[derive(Debug)]
pub struct ToplevelCtx {
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
}

impl ToplevelCtx {
    pub fn new() -> ToplevelCtx {
        ToplevelCtx {
            lvars: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct ClassCtx {
    /// Current namespace
    pub namespace: ClassFullname,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
}

impl ClassCtx {
    pub fn new(namespace: ClassFullname) -> ClassCtx {
        ClassCtx {
            namespace,
            lvars: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct MethodCtx {
    /// Signature of the current method
    signature: MethodSignature,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
    /// List of instance variables in an initializer found so far.
    /// Empty if the method is not `#initialize`
    pub iivars: SkIVars,
    /// List of inherited ivars
    /// Empty if the method is not `#initialize`
    pub super_ivars: SkIVars, // TODO: this can be just &'a SkIVars
}

impl MethodCtx {
    pub fn new(signature: MethodSignature, super_ivars: Option<SkIVars>) -> MethodCtx {
        MethodCtx {
            signature,
            lvars: Default::default(),
            iivars: Default::default(),
            super_ivars: super_ivars.unwrap_or(Default::default()),
        }
    }
}

#[derive(Debug)]
pub struct LambdaCtx {
    /// Parameters of the current lambda
    pub params: Vec<MethodParam>,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
    /// List of free variables captured in this context
    pub captures: Vec<LambdaCapture>,
}

impl LambdaCtx {
    pub fn new(params: Vec<MethodParam>) -> LambdaCtx {
        LambdaCtx {
            params,
            lvars: Default::default(),
            captures: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CtxKind {
    Toplevel,
    Class,
    Method,
    Lambda,
    While,
}

/// A local variable
#[derive(Debug)]
pub struct CtxLVar {
    pub name: String,
    pub ty: TermTy,
    pub readonly: bool,
}

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

impl HirMakerContext {
    /// Create initial ctx
    pub fn new() -> HirMakerContext {
        HirMakerContext {
            current: CtxKind::Toplevel,
            toplevel: ToplevelCtx::new(),
            classes: vec![],
            method: None,
            lambdas: vec![],
        }
    }

    /// Returns the current namespace
    pub fn namespace(&self) -> &str {
        if let Some(class_ctx) = self.classes.last() {
            &class_ctx.namespace.0
        } else {
            ""
        }
    }

    /// The type of `self` in the current scope
    pub fn self_ty(&self) -> TermTy {
        match self.current {
            CtxKind::Toplevel => ty::raw("Object"),
            CtxKind::Class => {
                let class_ctx = self.classes.last().unwrap();
                ty::meta(&class_ctx.namespace.0)
            }
            _ => {
                if let Some(class_ctx) = self.classes.last() {
                    ty::raw(&class_ctx.namespace.0)
                } else {
                    // This lambda is on the toplevel
                    ty::raw("Object")
                }
            }
        }
    }

    /// Iterates over lvar scopes starting from the current scope
    pub fn lvar_scopes(&self) -> LVarIter {
        LVarIter::new(self)
    }

    /// Add a local variable to current context
    pub fn declare_lvar(&mut self, name: &str, ty: TermTy, readonly: bool) {
        let lvars = match self.current {
            CtxKind::Toplevel => &mut self.toplevel.lvars,
            CtxKind::Class => &mut self.classes.last_mut().unwrap().lvars,
            CtxKind::Method => &mut self.method.as_mut().unwrap().lvars,
            CtxKind::Lambda => &mut self.lambdas.last_mut().unwrap().lvars,
            CtxKind::While => {
                if self.lambdas.len() > 0 {
                    &mut self.lambdas.last_mut().unwrap().lvars
                } else if self.method.is_some() {
                    &mut self.method.as_mut().unwrap().lvars
                } else if self.classes.len() > 0 {
                    &mut self.classes.last_mut().unwrap().lvars
                } else {
                    &mut self.toplevel.lvars
                }
            }
        };
        let k = name.to_string();
        let v = CtxLVar {
            name: name.to_string(),
            ty,
            readonly,
        };
        lvars.insert(k, v);
    }

    /// Returns if we're in an `#initialize`
    pub fn in_initializer(&self) -> bool {
        if let Some(method_ctx) = &self.method {
            method_ctx.signature.fullname.first_name.0 == "initialize"
        } else {
            false
        }
    }

    /// Push a LambdaCapture to captures
    pub fn push_lambda_capture(&mut self, cap: LambdaCapture) -> usize {
        let lambda_ctx = self.lambdas.last_mut().expect("not in lambda");
        lambda_ctx.captures.push(cap);
        lambda_ctx.captures.len() - 1
    }
}

/// Iterates over each lvar scope.
pub struct LVarIter<'a> {
    ctx: &'a HirMakerContext,
    cur: Option<CtxKind>,
    idx: usize,
}

impl<'a> LVarIter<'a> {
    fn new(ctx: &HirMakerContext) -> LVarIter {
        let c = ctx.current.clone();
        let (cur, idx) = match &c {
            CtxKind::Toplevel => (c, 0),
            CtxKind::Class => (c, ctx.classes.len() - 1),
            CtxKind::Method => (c, 0),
            CtxKind::Lambda => (c, ctx.lambdas.len() - 1),
            // `while` does not make a lvar scope, so find the nearest one
            CtxKind::While => {
                if !ctx.lambdas.is_empty() {
                    (CtxKind::Lambda, ctx.lambdas.len() - 1)
                } else if ctx.method.is_some() {
                    (CtxKind::Method, 0)
                } else if !ctx.classes.is_empty() {
                    (CtxKind::Class, ctx.classes.len() - 1)
                } else {
                    (CtxKind::Toplevel, 0)
                }
            }
        };
        LVarIter {
            ctx,
            cur: Some(cur),
            idx,
        }
    }
}

impl<'a> Iterator for LVarIter<'a> {
    /// Yields `(lvars, params, depth)`
    type Item = (&'a HashMap<String, CtxLVar>, &'a [MethodParam], isize);

    fn next(&mut self) -> Option<Self::Item> {
        match self.cur {
            // Toplevel -> end.
            Some(CtxKind::Toplevel) => {
                self.cur = None;
                Some((&self.ctx.toplevel.lvars, &[], -1))
            }
            // Classes -> end.
            Some(CtxKind::Class) => {
                let class_ctx = self.ctx.classes.get(self.idx).unwrap();
                if self.idx == 0 {
                    self.cur = None;
                } else {
                    self.idx -= 1;
                }
                Some((&class_ctx.lvars, &[], -1))
            }
            // Method -> end.
            Some(CtxKind::Method) => {
                self.cur = None;
                let method_ctx = self.ctx.method.as_ref().unwrap();
                Some((&method_ctx.lvars, &method_ctx.signature.params, -1))
            }
            // Lambdas -> (Method or Class or Toplevel)
            Some(CtxKind::Lambda) => {
                let orig_idx = self.idx as isize;
                let lambda_ctx = self.ctx.lambdas.get(self.idx).unwrap();
                if self.idx == 0 {
                    self.cur = if self.ctx.method.is_some() {
                        Some(CtxKind::Method)
                    } else if !self.ctx.classes.is_empty() {
                        self.idx = self.ctx.classes.len() - 1;
                        Some(CtxKind::Class)
                    } else {
                        Some(CtxKind::Toplevel)
                    }
                } else {
                    self.idx -= 1;
                }
                Some((&lambda_ctx.lvars, &lambda_ctx.params, orig_idx))
            }
            // ::new() never sets `While` to .cur
            Some(CtxKind::While) => panic!("must not happen"),
            None => None,
        }
    }
}

/// Destructively extract list of local variables
pub fn extract_lvars(lvars: &mut HashMap<String, CtxLVar>) -> HirLVars {
    std::mem::take(lvars)
        .into_iter()
        .map(|(name, ctx_lvar)| (name, ctx_lvar.ty))
        .collect::<Vec<_>>()
}

impl HirMaker {
    /// Returns type parameter of the current class
    pub(super) fn current_class_typarams(&self) -> Vec<String> {
        let typarams = &self
            .class_dict
            .find_class(&self.ctx.self_ty().fullname)
            .unwrap()
            .typarams;
        typarams.iter().map(|x| x.name.clone()).collect()
    }

    /// Returns type parameter of the current method
    pub(super) fn current_method_typarams(&self) -> Vec<String> {
        if let Some(method_ctx) = &self.ctx.method {
            method_ctx.signature.typarams.clone()
        } else {
            vec![]
        }
    }
}
