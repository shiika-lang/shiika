use crate::hir::hir_maker::HirMaker;
use crate::hir::*;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct HirMakerContext {
    /// Type of this ctx
    pub kind: CtxKind,
    /// Where this ctx is in the ctx_stack
    pub depth: usize,
    /// The type of current `self`
    pub self_ty: TermTy,
    /// Current namespace
    /// `""` for toplevel
    pub namespace: ClassFullname,
}

#[derive(Debug, PartialEq, Clone)]
pub enum CtxKind {
    Toplevel,
    Class,
    Method,
    Initializer,
    Lambda,
}

#[derive(Debug)]
pub struct HirMakerContext_ {
    /// Which kind of scope we're in
    pub current: CtxKind,
    // Context of each scope
    pub toplevel: ToplevelCtx,
    pub classes: Vec<ClassCtx>,
    pub method: Option<MethodCtx>,
    pub lambdas: Vec<LambdaCtx>,
}

impl HirMakerContext_ {
    /// Create initial ctx
    pub fn new() -> HirMakerContext_ {
        HirMakerContext_ {
            current: CtxKind::Toplevel,
            toplevel: ToplevelCtx::new(),
            classes: vec![],
            method: None,
            lambdas: vec![],
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
            _ => todo!(),
        };
        let k = name.to_string();
        let v = CtxLVar {
            name: name.to_string(),
            ty,
            readonly,
        };
        lvars.insert(k, v);
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
    ctx: &'a HirMakerContext_,
    cur: Option<CtxKind>,
    idx: usize,
}

impl<'a> LVarIter<'a> {
    fn new(ctx: &HirMakerContext_) -> LVarIter {
        let idx = match ctx.current {
            CtxKind::Toplevel => 0,
            CtxKind::Class => ctx.classes.len() - 1,
            CtxKind::Method => 0,
            CtxKind::Lambda => ctx.lambdas.len() - 1,
            _ => todo!(),
        };
        LVarIter {
            ctx,
            cur: Some(ctx.current.clone()),
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
            // Lambdas -> Method
            Some(CtxKind::Lambda) => {
                let orig_idx = self.idx as isize;
                let lambda_ctx = self.ctx.lambdas.get(self.idx).unwrap();
                if self.idx == 0 {
                    self.cur = if self.ctx.method.is_some() {
                        Some(CtxKind::Method)
                    } else if !self.ctx.classes.is_empty() {
                        Some(CtxKind::Class)
                    } else {
                        Some(CtxKind::Toplevel)
                    }
                } else {
                    self.idx -= 1;
                }
                Some((&lambda_ctx.lvars, &lambda_ctx.params, orig_idx))
            }
            None => None,
            _ => todo!(),
        }
    }
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
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
}

impl ClassCtx {
    pub fn new() -> ClassCtx {
        ClassCtx {
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

/// Destructively extract list of local variables
pub fn extract_lvars(lvars: &mut HashMap<String, CtxLVar>) -> HirLVars {
    std::mem::take(lvars)
        .into_iter()
        .map(|(name, ctx_lvar)| (name, ctx_lvar.ty))
        .collect::<Vec<_>>()
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
    /// Create a ctx for toplevel
    pub fn toplevel() -> HirMakerContext {
        // REVIEW: not sure this 'static is the right way
        HirMakerContext {
            kind: CtxKind::Toplevel,
            depth: 0,
            self_ty: ty::raw("Object"),
            namespace: ClassFullname("".to_string()),
        }
    }

    /// Create a class context
    pub fn class_ctx(fullname: &ClassFullname, depth: usize) -> HirMakerContext {
        HirMakerContext {
            kind: CtxKind::Class,
            depth,
            self_ty: ty::raw("Object"),
            namespace: fullname.clone(),
        }
    }

    /// Create a method context
    pub fn method_ctx(class_ctx: &HirMakerContext) -> HirMakerContext {
        HirMakerContext {
            kind: CtxKind::Method,
            depth: class_ctx.depth + 1,
            self_ty: ty::raw(&class_ctx.namespace.0),
            namespace: class_ctx.namespace.clone(),
        }
    }

    /// Create a initializer context
    pub fn initializer_ctx(class_ctx: &HirMakerContext) -> HirMakerContext {
        HirMakerContext {
            kind: CtxKind::Initializer,
            depth: class_ctx.depth + 1,
            self_ty: ty::raw(&class_ctx.namespace.0),
            namespace: class_ctx.namespace.clone(),
        }
    }

    /// Create a ctx for lambda
    pub fn lambda_ctx(method_ctx: &HirMakerContext) -> HirMakerContext {
        HirMakerContext {
            kind: CtxKind::Lambda,
            depth: method_ctx.depth + 1,
            self_ty: method_ctx.self_ty.clone(),
            namespace: method_ctx.namespace.clone(),
        }
    }
}

impl HirMaker {
    pub(super) fn ctx(&self) -> &HirMakerContext {
        self.ctx_stack.last().unwrap()
    }

    pub(super) fn push_ctx(&mut self, ctx: HirMakerContext) {
        self.ctx_stack.push(ctx);
    }

    pub(super) fn pop_ctx(&mut self) -> HirMakerContext {
        self.ctx_stack.pop().unwrap()
    }

    /// Returns depth of next ctx
    pub(super) fn next_ctx_depth(&self) -> usize {
        self.ctx_stack.len()
    }

    pub(super) fn method_ctx(&self) -> Option<&HirMakerContext> {
        let opt_idx = self
            .find_ctx(CtxKind::Method)
            .or_else(|| self.find_ctx(CtxKind::Initializer));
        opt_idx.map(|i| &self.ctx_stack[i])
    }

    /// Find nearest enclosing ctx of the `kind`
    fn find_ctx(&self, kind: CtxKind) -> Option<usize> {
        let mut i = (self.ctx_stack.len() as isize) - 1;
        while i >= 0 {
            let ctx = &self.ctx_stack[i as usize];
            if ctx.kind == kind {
                return Some(i as usize);
            }
            i -= 1
        }
        None
    }

    /// Returns type parameter of the current class
    pub(super) fn current_class_typarams(&self) -> Vec<String> {
        let typarams = &self
            .class_dict
            .find_class(&self.ctx().self_ty.fullname)
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
