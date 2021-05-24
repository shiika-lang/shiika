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
    pub namespace: Namespace,
    /// Names of class type parameters
    pub typarams: Vec<String>,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
}

impl ClassCtx {
    pub fn new(namespace: Namespace, typarams: Vec<String>) -> ClassCtx {
        ClassCtx {
            namespace,
            typarams,
            lvars: Default::default(),
        }
    }
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

impl MethodCtx {
    pub fn new(signature: MethodSignature, super_ivars: Option<SkIVars>) -> MethodCtx {
        MethodCtx {
            signature,
            lvars: Default::default(),
            iivars: Default::default(),
            super_ivars: super_ivars.unwrap_or_default(),
        }
    }
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
    pub fn new(is_fn: bool, params: Vec<MethodParam>) -> LambdaCtx {
        LambdaCtx {
            is_fn,
            params,
            lvars: Default::default(),
            captures: Default::default(),
            has_break: false,
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

    /// Returns true if current context is a fn
    pub fn current_is_fn(&self) -> bool {
        self.current == CtxKind::Lambda && self.lambdas.last().unwrap().is_fn
    }

    /// Returns a debugging string like "toplevel", "Class1", "Class1#method1", etc.
    pub fn describe_current_place(&self) -> String {
        if let Some(method_ctx) = &self.method {
            method_ctx.signature.fullname.to_string()
        } else {
            match self.current {
                CtxKind::Toplevel => "toplevel".to_string(),
                CtxKind::Class => self.classes.last().unwrap().namespace.to_string(),
                CtxKind::Lambda => "lambda".to_string(),
                _ => panic!("must not happen"),
            }
        }
    }

    /// Set `c` to `self.current` and the original value to `c`
    pub fn swap_current(&mut self, c: &mut CtxKind) {
        std::mem::swap(c, &mut self.current);
    }

    /// Returns the nearest lambda ctx
    pub fn lambda_mut(&mut self) -> &mut LambdaCtx {
        self.lambdas.last_mut().unwrap()
    }

    /// The type of `self` in the current scope
    pub fn self_ty(&self) -> TermTy {
        match self.current {
            CtxKind::Toplevel => ty::raw("Object"),
            CtxKind::Class => {
                let class_ctx = self.classes.last().unwrap();
                ty::meta(&class_ctx.namespace.to_string())
            }
            _ => {
                if let Some(class_ctx) = self.classes.last() {
                    ty::raw(&class_ctx.namespace.to_string())
                } else {
                    // This lambda is on the toplevel
                    ty::raw("Object")
                }
            }
        }
    }

    /// Iterates over constant scopes starting from the current one
    pub fn const_scopes(&self) -> NamespaceIter {
        NamespaceIter::new(self)
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
                if !self.lambdas.is_empty() {
                    &mut self.lambdas.last_mut().unwrap().lvars
                } else if self.method.is_some() {
                    &mut self.method.as_mut().unwrap().lvars
                } else if !self.classes.is_empty() {
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

    /// If there is a method or class typaram named `name`, returns its type
    pub fn lookup_typaram(&self, name: &str) -> Option<TermTy> {
        if let Some(method_ctx) = &self.method {
            let typarams = &method_ctx.signature.typarams;
            if let Some(i) = typarams.iter().position(|s| *name == *s) {
                return Some(ty::typaram(name, ty::TyParamKind::Method, i));
            }
            if let Some(class_ctx) = self.classes.last() {
                if method_ctx.signature.fullname.is_class_method() {
                    return None;
                }
                let typarams = &class_ctx.typarams;
                if let Some(i) = typarams.iter().position(|s| *name == *s) {
                    return Some(ty::typaram(name, ty::TyParamKind::Class, i));
                }
            }
        }
        None
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

/// Iterates over each constant scope.
pub struct NamespaceIter<'a> {
    ctx: &'a HirMakerContext,
    cur: Option<CtxKind>,
    idx: usize,
}

impl<'a> NamespaceIter<'a> {
    fn new(ctx: &HirMakerContext) -> NamespaceIter {
        let c = ctx.current.clone();
        let (cur, idx) = match &c {
            CtxKind::Toplevel => (c, 0),
            CtxKind::Class => (c, ctx.classes.len() - 1),
            CtxKind::Method => (CtxKind::Class, ctx.classes.len() - 1),
            // These does not make a constant scope, so find the nearest one
            CtxKind::Lambda | CtxKind::While => {
                if !ctx.classes.is_empty() {
                    (CtxKind::Class, ctx.classes.len() - 1)
                } else {
                    (CtxKind::Toplevel, 0)
                }
            }
        };
        NamespaceIter {
            ctx,
            cur: Some(cur),
            idx,
        }
    }
}

impl<'a> Iterator for NamespaceIter<'a> {
    /// Yields namespace
    type Item = Namespace;

    fn next(&mut self) -> Option<Self::Item> {
        match self.cur {
            // Toplevel -> end.
            Some(CtxKind::Toplevel) => {
                self.cur = None;
                Some(Namespace::root())
            }
            // Classes -> Toplevel
            Some(CtxKind::Class) => {
                let class_ctx = self.ctx.classes.get(self.idx).unwrap();
                if self.idx == 0 {
                    self.cur = Some(CtxKind::Toplevel);
                } else {
                    self.idx -= 1;
                }
                Some(class_ctx.namespace.clone())
            }
            Some(_) => panic!("must not happen"),
            None => None,
        }
    }
}

// REFACTOR: Move to HirMakerContext
impl<'hir_maker> HirMaker<'hir_maker> {
    /// Returns type parameter of the current class
    pub(super) fn current_class_typarams(&self) -> Vec<String> {
        if let Some(class_ctx) = self.ctx.classes.last() {
            if let Some(method_ctx) = &self.ctx.method {
                if !method_ctx.signature.fullname.is_class_method() {
                    return class_ctx.typarams.clone();
                }
            }
        }
        vec![]
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
