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
    /// Signature of the current method (Used to get the list of parameters)
    /// None if out of a method
    pub method_sig: Option<MethodSignature>,
    /// The type of current `self`
    pub self_ty: TermTy,
    /// Current namespace
    /// `""` for toplevel
    pub namespace: ClassFullname,
    /// Current local variables
    pub lvars: HashMap<String, CtxLVar>,
    /// List of free variables captured in this context
    pub captures: Vec<LambdaCapture>,

    //
    // ivar-related stuffs
    //
    /// List of instance variables in an initializer found so far
    pub iivars: SkIVars,
    /// Whether we are in an initializer
    pub is_initializer: bool,
    /// Number of inherited ivars. Only used when is_initializer is true
    pub super_ivars: SkIVars, // TODO: this can be just &'a SkIVars
}

#[derive(Debug, PartialEq)]
pub enum CtxKind {
    Toplevel,
    Class,
    Method,
    Lambda,
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
    pub ctx_depth: usize,
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
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: ClassFullname("".to_string()),
            lvars: HashMap::new(),
            captures: vec![],
            iivars: HashMap::new(),
            is_initializer: false,
            super_ivars: HashMap::new(),
        }
    }

    /// Create a class context
    pub fn class_ctx(fullname: &ClassFullname, depth: usize) -> HirMakerContext {
        HirMakerContext {
            kind: CtxKind::Class,
            depth,
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: fullname.clone(),
            lvars: HashMap::new(),
            captures: vec![],
            iivars: HashMap::new(),
            is_initializer: false,
            super_ivars: HashMap::new(),
        }
    }

    /// Create a method context
    pub fn method_ctx(
        class_ctx: &HirMakerContext,
        method_sig: &MethodSignature,
        is_initializer: bool,
        super_ivars: SkIVars,
    ) -> HirMakerContext {
        HirMakerContext {
            kind: CtxKind::Method,
            depth: class_ctx.depth + 1,
            method_sig: Some(method_sig.clone()),
            self_ty: ty::raw(&class_ctx.namespace.0),
            namespace: class_ctx.namespace.clone(),
            lvars: HashMap::new(),
            captures: vec![],
            iivars: HashMap::new(),
            is_initializer,
            super_ivars,
        }
    }

    /// Create a ctx for lambda
    pub fn lambda_ctx(method_ctx: &HirMakerContext, params: Vec<MethodParam>) -> HirMakerContext {
        let sig = MethodSignature {
            fullname: method_fullname(&class_fullname("(anon)"), "(anon)"),
            ret_ty: ty::raw("(dummy)"),
            params,
        };
        HirMakerContext {
            kind: CtxKind::Lambda,
            depth: method_ctx.depth + 1,
            method_sig: Some(sig),
            self_ty: method_ctx.self_ty.clone(),
            namespace: method_ctx.namespace.clone(),
            lvars: HashMap::new(),
            captures: vec![],
            iivars: HashMap::new(),
            is_initializer: false,
            super_ivars: HashMap::new(),
        }
    }

    /// Return local variable of given name, if any
    pub fn find_lvar(&self, name: &str) -> Option<&CtxLVar> {
        self.lvars.get(name)
    }

    /// Destructively extract list of local variables
    pub fn extract_lvars(&mut self) -> HirLVars {
        std::mem::take(&mut self.lvars)
            .into_iter()
            .map(|(name, ctx_lvar)| (name, ctx_lvar.ty))
            .collect::<Vec<_>>()
    }

    /// Return method/lambda argument of given name, if any
    pub fn find_fn_arg(&self, name: &str) -> Option<(usize, &MethodParam)> {
        self.method_sig
            .as_ref()
            .map(|sig| sig.find_param(name))
            .flatten()
    }
}

impl HirMaker {
    pub(super) fn ctx(&self) -> &HirMakerContext {
        self.ctx_stack.last().unwrap()
    }

    pub(super) fn ctx_mut(&mut self) -> &mut HirMakerContext {
        self.ctx_stack.last_mut().unwrap()
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
        let mut i = (self.ctx_stack.len() as isize) - 1;
        while i >= 0 {
            let ctx = &self.ctx_stack[i as usize];
            if ctx.kind == CtxKind::Method {
                return Some(ctx);
            }
            i -= 1
        }
        None
    }

    pub(super) fn method_ctx_mut(&mut self) -> Option<&mut HirMakerContext> {
        let mut i = (self.ctx_stack.len() as isize) - 1;
        while i >= 0 {
            let ctx = &self.ctx_stack[i as usize];
            if ctx.kind == CtxKind::Method {
                return Some(&mut self.ctx_stack[i as usize]);
            }
            i -= 1
        }
        None
    }

    pub(super) fn outer_lvar_scope_of(&self, ctx: &HirMakerContext) -> Option<&HirMakerContext> {
        if ctx.kind != CtxKind::Lambda { return None }
        if ctx.depth == 0 { return None }
        let outer_ctx = &self.ctx_stack[ctx.depth - 1];
        Some(outer_ctx)
    }
}
