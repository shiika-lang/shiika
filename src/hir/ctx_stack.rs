use crate::hir::hir_maker_context::*;
use crate::hir::MethodParam;
use crate::names::Namespace;
use crate::ty;
use crate::ty::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct CtxStack(Vec<HirMakerContext>);

impl CtxStack {
    /// Create a CtxStack
    pub fn new(v: Vec<HirMakerContext>) -> CtxStack {
        CtxStack(v)
    }

    /// Returns length of stack
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns nth item
    pub fn get(&self, idx: usize) -> &HirMakerContext {
        &self.0[idx]
    }

    /// Push a ctx
    pub fn push(&mut self, c: HirMakerContext) {
        self.0.push(c);
    }

    /// Pop a ctx
    pub fn pop(&mut self) -> HirMakerContext {
        self.0.pop().expect("[BUG] no ctx to pop")
    }

    /// Pop the ToplevelCtx on the stack top
    pub fn pop_toplevel_ctx(&mut self) -> ToplevelCtx {
        if let HirMakerContext::Toplevel(toplevel_ctx) = self.pop() {
            toplevel_ctx
        } else {
            panic!("[BUG] top is not ToplevelCtx");
        }
    }

    /// Pop the MethodCtx on the stack top
    pub fn pop_method_ctx(&mut self) -> MethodCtx {
        if let HirMakerContext::Method(method_ctx) = self.pop() {
            method_ctx
        } else {
            panic!("[BUG] top is not MethodCtx");
        }
    }

    /// Pop the MethodCtx on the stack top
    pub fn pop_lambda_ctx(&mut self) -> LambdaCtx {
        if let HirMakerContext::Lambda(lambda_ctx) = self.pop() {
            lambda_ctx
        } else {
            panic!("[BUG] top is not LambdaCtx");
        }
    }

    /// Returns the ctx on the top of the stack
    pub fn top(&self) -> &HirMakerContext {
        // ctx_stack will not be empty because toplevel ctx is always there
        self.0.last().expect("[BUG] ctx_stack is empty")
    }

    /// Returns the ctx on the top of the stack
    pub fn top_mut(&mut self) -> &mut HirMakerContext {
        // ctx_stack will not be empty because toplevel ctx is always there
        self.0.last_mut().expect("[BUG] ctx_stack is empty")
    }

    /// Return nearest enclosing class ctx, if any
    pub fn class_ctx(&self) -> Option<&ClassCtx> {
        for x in self.0.iter().rev() {
            if let HirMakerContext::Class(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return enclosing method ctx, if any
    pub fn method_ctx(&self) -> Option<&MethodCtx> {
        for x in self.0.iter().rev() {
            if let HirMakerContext::Method(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return enclosing method ctx, if any
    pub fn method_ctx_mut(&mut self) -> Option<&mut MethodCtx> {
        for x in self.0.iter_mut().rev() {
            if let HirMakerContext::Method(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return ctx of nearest enclosing lambda, if any
    pub fn lambda_ctx(&self) -> Option<&LambdaCtx> {
        for x in self.0.iter().rev() {
            if let HirMakerContext::Lambda(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return ctx of nearest enclosing lambda, if any
    pub fn lambda_ctx_mut(&mut self) -> Option<&mut LambdaCtx> {
        for x in self.0.iter_mut().rev() {
            if let HirMakerContext::Lambda(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Returns a debugging string like "toplevel", "Class1", "Class1#method1", etc.
    pub fn describe_current_place(&self) -> String {
        if let Some(method_ctx) = self.method_ctx() {
            method_ctx.signature.fullname.to_string()
        } else {
            match self.top() {
                HirMakerContext::Toplevel(_) => "toplevel".to_string(),
                HirMakerContext::Class(class_ctx) => class_ctx.namespace.string(),
                HirMakerContext::Method(method_ctx) => method_ctx.signature.fullname.to_string(),
                HirMakerContext::Lambda(_) => "lambda".to_string(),
                HirMakerContext::While(_) => "while".to_string(),
            }
        }
    }

    /// The type of `self` in the current scope
    pub fn self_ty(&self) -> TermTy {
        if let Some(class_ctx) = self.class_ctx() {
            if let Some(_) = self.method_ctx() {
                ty::raw(&class_ctx.namespace.string())
            } else {
                ty::meta(&class_ctx.namespace.string())
            }
        } else {
            // This lambda is on the toplevel
            ty::raw("Object")
        }
    }

    /// Add a local variable to current context
    pub fn declare_lvar(&mut self, name: &str, ty: TermTy, readonly: bool) {
        let lvars = self.current_lvars_mut();
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
        if let Some(method_ctx) = self.method_ctx() {
            method_ctx.signature.fullname.first_name.0 == "initialize"
        } else {
            false
        }
    }

    /// Push a LambdaCapture to captures
    pub fn push_lambda_capture(&mut self, cap: LambdaCapture) -> usize {
        let lambda_ctx = self.lambda_ctx_mut().expect("not in lambda");
        lambda_ctx.captures.push(cap);
        lambda_ctx.captures.len() - 1
    }

    /// Returns type parameter of the current class
    pub fn current_class_typarams(&self) -> Vec<String> {
        if let Some(class_ctx) = self.class_ctx() {
            if let Some(method_ctx) = self.method_ctx() {
                if !method_ctx.signature.fullname.is_class_method() {
                    return class_ctx.typarams.clone();
                }
            }
        }
        vec![]
    }

    /// Returns type parameter of the current method
    pub fn current_method_typarams(&self) -> Vec<String> {
        if let Some(method_ctx) = self.method_ctx() {
            method_ctx.signature.typarams.clone()
        } else {
            vec![]
        }
    }

    /// If there is a method or class typaram named `name`, returns its type
    pub fn lookup_typaram(&self, name: &str) -> Option<TermTy> {
        if let Some(method_ctx) = self.method_ctx() {
            let typarams = &method_ctx.signature.typarams;
            if let Some(i) = typarams.iter().position(|s| *name == *s) {
                return Some(ty::typaram(name, ty::TyParamKind::Method, i));
            }
            if let Some(class_ctx) = self.class_ctx() {
                if method_ctx.signature.fullname.is_class_method() {
                    return None;
                }
                let typarams = &class_ctx.typarams;
                if let Some(i) = typarams.iter().position(|s| *name == *s) {
                    return Some(ty::typaram(name, ty::TyParamKind::Class, i));
                }
            }
        }
        // No typarams are accessible outside methods
        None
    }

    /// Iterates over lvar scopes starting from the current scope
    pub fn lvar_scopes(&self) -> LVarIter {
        LVarIter::new(self)
    }

    pub fn current_lvars_mut(&mut self) -> &mut CtxLVars {
        for ctx in self.0.iter_mut().rev() {
            if let Some(lvars) = ctx.opt_lvars() {
                return lvars;
            }
        }
        panic!("[BUG] current lvars not found")
    }

    /// Iterates over constant scopes starting from the current one
    pub fn const_scopes(&self) -> NamespaceIter {
        NamespaceIter::new(self)
    }
}

/// Iterates over each lvar scope.
pub struct LVarIter<'hir_maker> {
    ctx_stack: &'hir_maker CtxStack,
    cur: usize,
    finished: bool,
}

impl<'hir_maker> LVarIter<'hir_maker> {
    fn new(ctx_stack: &CtxStack) -> LVarIter {
        let mut finished = false;
        let mut cur = ctx_stack.len();
        loop {
            if cur == 0 {
                finished = true;
                break;
            }
            cur -= 1;
            match ctx_stack.get(cur) {
                HirMakerContext::Toplevel(_)
                | HirMakerContext::Class(_)
                | HirMakerContext::Method(_)
                | HirMakerContext::Lambda(_) => break,
                // Does not make lvar scope
                HirMakerContext::While(_) => (),
            }
        }
        LVarIter {
            ctx_stack,
            cur,
            finished,
        }
    }
}

impl<'a> Iterator for LVarIter<'a> {
    /// Yields `(lvars, params, depth)`
    type Item = (&'a HashMap<String, CtxLVar>, &'a [MethodParam], isize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.ctx_stack.get(self.cur) {
            // Toplevel -> end.
            HirMakerContext::Toplevel(toplevel_ctx) => {
                self.finished = true;
                Some((&toplevel_ctx.lvars, &[], -1))
            }
            // Classes -> end.
            HirMakerContext::Class(class_ctx) => {
                self.finished = true;
                Some((&class_ctx.lvars, &[], -1))
            }
            // Method -> end.
            HirMakerContext::Method(method_ctx) => {
                self.finished = true;
                Some((&method_ctx.lvars, &method_ctx.signature.params, -1))
            }
            // Lambdas -> (Method or Class or Toplevel)
            HirMakerContext::Lambda(lambda_ctx) => {
                let orig_idx = self.cur;
                self.cur -= 1;
                Some((&lambda_ctx.lvars, &lambda_ctx.params, orig_idx as isize))
            }
            // ::new() never sets `While` to .cur
            HirMakerContext::While(_) => panic!("must not happen"),
        }
    }
}

/// Iterates over each constant scope.
pub struct NamespaceIter<'hir_maker> {
    ctx_stack: &'hir_maker CtxStack,
    cur: usize,
    finished: bool,
}

impl<'hir_maker> NamespaceIter<'hir_maker> {
    fn new(ctx_stack: &CtxStack) -> NamespaceIter {
        let mut finished = false;
        let mut cur = ctx_stack.len();
        loop {
            if cur == 0 {
                finished = true;
                break;
            }
            cur -= 1;
            match ctx_stack.get(cur) {
                HirMakerContext::Toplevel(_) | HirMakerContext::Class(_) => break,
                // Does not make constant scope
                HirMakerContext::Method(_)
                | HirMakerContext::Lambda(_)
                | HirMakerContext::While(_) => (),
            }
        }
        NamespaceIter {
            ctx_stack,
            cur,
            finished,
        }
    }
}

impl<'a> Iterator for NamespaceIter<'a> {
    /// Yields namespace
    type Item = Namespace;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        for idx in (0..=self.cur).rev() {
            match self.ctx_stack.get(idx) {
                HirMakerContext::Toplevel(_) => {
                    self.finished = true;
                    return Some(Namespace::root());
                }
                HirMakerContext::Class(class_ctx) => {
                    self.cur -= 1;
                    return Some(class_ctx.namespace.clone());
                }
                _ => (), // Skip this ctx
            }
        }
        panic!("[BUG] no more namespace");
    }
}