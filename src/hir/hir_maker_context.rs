use crate::hir::*;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use std::collections::HashMap;

static mut LAST_CTX_ID: usize = 0;

#[derive(Debug)]
pub struct HirMakerContext<'make> {
    /// Unique number to denote this ctx
    pub id: usize,
    /// Next surrounding ctx (if any)
    pub outer_ctx: Option<&'make HirMakerContext<'make>>,
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

impl<'make> PartialEq for HirMakerContext<'make> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<'make> HirMakerContext<'make> {
    /// Create a ctx for toplevel
    pub fn toplevel() -> HirMakerContext<'static> {
        // REVIEW: not sure this 'static is the right way
        HirMakerContext {
            outer_ctx: None,
            id: new_id(),
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
    pub fn class_ctx(fullname: &ClassFullname) -> HirMakerContext {
        HirMakerContext {
            outer_ctx: None,
            id: new_id(),
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
        class_ctx: &'make HirMakerContext,
        method_sig: &MethodSignature,
        is_initializer: bool,
    ) -> HirMakerContext<'make> {
        HirMakerContext {
            outer_ctx: Some(class_ctx),
            id: new_id(),
            method_sig: Some(method_sig.clone()),
            self_ty: ty::raw(&class_ctx.namespace.0),
            namespace: class_ctx.namespace.clone(),
            lvars: HashMap::new(),
            captures: vec![],
            iivars: HashMap::new(),
            is_initializer,
            super_ivars: HashMap::new(),
        }
    }

    /// Create a ctx for lambda
    pub fn lambda_ctx(
        method_ctx: &'make HirMakerContext,
        params: Vec<MethodParam>,
    ) -> HirMakerContext<'make> {
        let sig = MethodSignature {
            fullname: method_fullname(&class_fullname("(anon)"), "(anon)"),
            ret_ty: ty::raw("(dummy)"),
            params,
        };
        HirMakerContext {
            outer_ctx: Some(method_ctx),
            id: new_id(),
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

    /// Return method/lambda argument of given name, if any
    pub fn find_fn_arg(&self, name: &str) -> Option<(usize, &MethodParam)> {
        self.method_sig
            .as_ref()
            .map(|sig| sig.find_param(name))
            .flatten()
    }
}

/// Return a newly created ctx id
fn new_id() -> usize {
    unsafe {
        LAST_CTX_ID += 1;
        LAST_CTX_ID
    }
}

/// A local variable
#[derive(Debug)]
pub struct CtxLVar {
    pub name: String,
    pub ty: TermTy,
    pub readonly: bool,
}
