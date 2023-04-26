use crate::hir_maker_context::*;
use shiika_core::names::Namespace;
use shiika_core::{ty, ty::*};
use skc_hir::MethodParam;
use std::collections::HashMap;

#[derive(Debug)]
pub struct CtxStack {
    /// List of ctxs
    vec: Vec<HirMakerContext>,
}

impl CtxStack {
    /// Create a CtxStack
    pub fn new(v: Vec<HirMakerContext>) -> CtxStack {
        CtxStack { vec: v }
    }

    /// Returns length of stack
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns nth item
    pub fn get(&self, idx: usize) -> &HirMakerContext {
        &self.vec[idx]
    }

    /// Returns nth item
    pub fn get_mut(&mut self, idx: usize) -> &mut HirMakerContext {
        &mut self.vec[idx]
    }

    /// Push a ctx
    pub fn push(&mut self, c: HirMakerContext) {
        self.vec.push(c);
    }

    /// Pop a ctx
    fn pop(&mut self) -> HirMakerContext {
        let c = self.vec.pop().expect("[BUG] no ctx to pop");
        c
    }

    /// Pop the ToplevelCtx on the stack top
    pub fn pop_toplevel_ctx(&mut self) -> ToplevelCtx {
        if let HirMakerContext::Toplevel(toplevel_ctx) = self.pop() {
            toplevel_ctx
        } else {
            panic!("[BUG] top is not ToplevelCtx");
        }
    }

    /// Pop the ToplevelCtx on the stack top
    pub fn pop_class_ctx(&mut self) -> ClassCtx {
        if let HirMakerContext::Class(class_ctx) = self.pop() {
            class_ctx
        } else {
            panic!("[BUG] top is not ClassCtx");
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

    /// Pop the LambdaCtx on the stack top
    pub fn pop_lambda_ctx(&mut self) -> LambdaCtx {
        if let HirMakerContext::Lambda(lambda_ctx) = self.pop() {
            lambda_ctx
        } else {
            panic!("[BUG] top is not LambdaCtx");
        }
    }

    /// Pop the WhileCtx on the stack top
    pub fn pop_while_ctx(&mut self) -> WhileCtx {
        if let HirMakerContext::While(ctx) = self.pop() {
            ctx
        } else {
            panic!("[BUG] top is not WhileCtx");
        }
    }

    pub fn pop_if_ctx(&mut self) -> IfCtx {
        if let HirMakerContext::If(ctx) = self.pop() {
            ctx
        } else {
            panic!("[BUG] top is not IfCtx")
        }
    }

    /// Pop the MatchClauseCtx on the stack top
    pub fn pop_match_clause_ctx(&mut self) -> MatchClauseCtx {
        if let HirMakerContext::MatchClause(ctx) = self.pop() {
            ctx
        } else {
            panic!("[BUG] top is not MatchClauseCtx");
        }
    }

    /// Returns the ctx on the top of the stack
    pub fn top(&self) -> &HirMakerContext {
        // ctx_stack will not be empty because toplevel ctx is always there
        self.vec.last().expect("[BUG] ctx_stack is empty")
    }

    /// Return nearest enclosing class ctx, if any
    pub fn class_ctx(&self) -> Option<&ClassCtx> {
        for x in self.vec.iter().rev() {
            if let HirMakerContext::Class(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return enclosing method ctx, if any
    pub fn method_ctx(&self) -> Option<&MethodCtx> {
        for x in self.vec.iter().rev() {
            if let HirMakerContext::Method(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return enclosing method ctx, if any
    pub fn method_ctx_mut(&mut self) -> Option<&mut MethodCtx> {
        for x in self.vec.iter_mut().rev() {
            if let HirMakerContext::Method(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return ctx of nearest enclosing lambda, if any
    pub fn lambda_ctx(&self) -> Option<&LambdaCtx> {
        for x in self.vec.iter().rev() {
            if let HirMakerContext::Lambda(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return ctx of nearest enclosing lambda, if any
    pub fn lambda_ctx_mut(&mut self) -> Option<&mut LambdaCtx> {
        for x in self.vec.iter_mut().rev() {
            if let HirMakerContext::Lambda(c) = x {
                return Some(c);
            }
        }
        None
    }

    /// Return ctx of nearest enclosing loop, if any
    pub fn loop_ctx_mut(&mut self) -> Option<&mut HirMakerContext> {
        for x in self.vec.iter_mut().rev() {
            if matches!(x, HirMakerContext::Lambda(_) | HirMakerContext::While(_)) {
                return Some(x);
            }
        }
        None
    }

    /// Returns a string like "toplevel", "Class1", "Class1#method1", etc.
    /// For debugging (of Shiika compiler.)
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
                HirMakerContext::If(_) => "if".to_string(),
                HirMakerContext::MatchClause(_) => "match".to_string(),
            }
        }
    }

    /// The type of `self` in the current scope
    pub fn self_ty(&self) -> TermTy {
        if let Some(class_ctx) = self.class_ctx() {
            if let Some(method_ctx) = self.method_ctx() {
                if method_ctx.signature.is_class_method() {
                    ty::meta(&class_ctx.namespace.string())
                } else {
                    let classname = &method_ctx.signature.fullname.type_name;
                    ty::return_type_of_new(classname, &class_ctx.typarams)
                }
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
            captured: false,
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

    /// Returns type parameter of the current class
    pub fn current_class_typarams(&self) -> Vec<TyParam> {
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
    pub fn current_method_typarams(&self) -> Vec<TyParam> {
        if let Some(method_ctx) = self.method_ctx() {
            method_ctx.signature.typarams.clone()
        } else {
            vec![]
        }
    }

    /// If there is a method or class typaram named `name`, returns its type
    pub fn lookup_typaram(&self, name: &str) -> Option<TyParamRef> {
        if let Some(method_ctx) = self.method_ctx() {
            let typarams = &method_ctx.signature.typarams;
            if let Some(i) = typarams.iter().position(|t| *name == *t.name) {
                return Some(ty::typaram_ref(name, ty::TyParamKind::Method, i));
            }
            if let Some(class_ctx) = self.class_ctx() {
                if method_ctx.signature.fullname.is_class_method() {
                    return None;
                }
                let typarams = &class_ctx.typarams;
                if let Some(i) = typarams.iter().position(|t| *name == *t.name) {
                    return Some(ty::typaram_ref(name, ty::TyParamKind::Class, i));
                }
            }
        }
        // No typarams are accessible outside methods
        None
    }

    /// Iterates over lvar scopes starting from the current scope
    pub fn lvar_scopes<'hir_maker>(&'hir_maker self) -> LVarIter<'hir_maker> {
        LVarIter::new(self)
    }

    pub fn current_lvars_mut(&mut self) -> &mut CtxLVars {
        for ctx in self.vec.iter_mut().rev() {
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
                | HirMakerContext::If(_)
                | HirMakerContext::Class(_)
                | HirMakerContext::Method(_)
                | HirMakerContext::Lambda(_)
                | HirMakerContext::MatchClause(_) => break,
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

pub struct LVarScope<'hir_maker> {
    pub ctx_idx: usize,
    pub lvars: &'hir_maker HashMap<String, CtxLVar>,
    pub params: &'hir_maker [MethodParam],
    pub is_lambda_scope: bool,
}

impl<'hir_maker> Iterator for LVarIter<'hir_maker> {
    type Item = LVarScope<'hir_maker>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.ctx_stack.get(self.cur) {
            // Toplevel -> end.
            HirMakerContext::Toplevel(toplevel_ctx) => {
                self.finished = true;
                Some(LVarScope {
                    ctx_idx: self.cur,
                    lvars: &toplevel_ctx.lvars,
                    params: &[],
                    is_lambda_scope: false,
                })
            }
            // Classes -> end.
            HirMakerContext::Class(class_ctx) => {
                self.finished = true;
                Some(LVarScope {
                    ctx_idx: self.cur,
                    lvars: &class_ctx.lvars,
                    params: &[],
                    is_lambda_scope: false,
                })
            }
            // Method -> end.
            HirMakerContext::Method(method_ctx) => {
                self.finished = true;
                Some(LVarScope {
                    ctx_idx: self.cur,
                    lvars: &method_ctx.lvars,
                    params: &method_ctx.signature.params,
                    is_lambda_scope: false,
                })
            }
            HirMakerContext::Lambda(lambda_ctx) => {
                let scope = LVarScope {
                    ctx_idx: self.cur,
                    lvars: &lambda_ctx.lvars,
                    params: &lambda_ctx.params,
                    is_lambda_scope: true,
                };
                self.cur -= 1;
                Some(scope)
            }
            HirMakerContext::If(if_ctx) => {
                let scope = LVarScope {
                    ctx_idx: self.cur,
                    lvars: &if_ctx.lvars,
                    params: &[],
                    is_lambda_scope: false,
                };
                self.cur -= 1;
                Some(scope)
            }
            HirMakerContext::MatchClause(match_clause_ctx) => {
                let scope = LVarScope {
                    ctx_idx: self.cur,
                    lvars: &match_clause_ctx.lvars,
                    params: &[],
                    is_lambda_scope: false,
                };
                self.cur -= 1;
                Some(scope)
            }
            // ::new() never sets `While` to .cur
            HirMakerContext::While(while_ctx) => {
                let scope = LVarScope {
                    ctx_idx: self.cur,
                    lvars: &while_ctx.lvars,
                    params: &[],
                    is_lambda_scope: false,
                };
                self.cur -= 1;
                Some(scope)
            }
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
                | HirMakerContext::MatchClause(_)
                | HirMakerContext::If(_)
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
                // Does not make constant scope
                HirMakerContext::Method(_)
                | HirMakerContext::Lambda(_)
                | HirMakerContext::MatchClause(_)
                | HirMakerContext::If(_)
                | HirMakerContext::While(_) => (),
            }
        }
        panic!("[BUG] no more namespace");
    }
}
