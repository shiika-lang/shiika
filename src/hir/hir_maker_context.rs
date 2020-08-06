use crate::hir::*;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct HirMakerContext {
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

impl HirMakerContext {
    /// Create a ctx for toplevel
    pub fn toplevel() -> HirMakerContext {
        HirMakerContext {
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: ClassFullname("".to_string()),
            lvars: HashMap::new(),
            iivars: HashMap::new(),
            is_initializer: false,
            super_ivars: HashMap::new(),
        }
    }

    /// Create a class context
    pub fn class_ctx(fullname: &ClassFullname) -> HirMakerContext {
        HirMakerContext {
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: fullname.clone(),
            lvars: HashMap::new(),
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
    ) -> HirMakerContext {
        HirMakerContext {
            method_sig: Some(method_sig.clone()),
            self_ty: ty::raw(&class_ctx.namespace.0),
            namespace: class_ctx.namespace.clone(),
            lvars: HashMap::new(),
            iivars: HashMap::new(),
            is_initializer,
            super_ivars: HashMap::new(),
        }
    }

    /// Create a ctx for lambda
    pub fn lambda_ctx(
        method_ctx: &HirMakerContext,
        lambda_sig: MethodSignature,
    ) -> HirMakerContext {
        HirMakerContext {
            method_sig: Some(lambda_sig),
            self_ty: method_ctx.self_ty.clone(),
            namespace: method_ctx.namespace.clone(),
            lvars: HashMap::new(),
            iivars: HashMap::new(),
            is_initializer: false,
            super_ivars: HashMap::new(),
        }
    }
}

/// A local variable
#[derive(Debug)]
pub struct CtxLVar {
    pub name: String,
    pub ty: TermTy,
    pub readonly: bool,
}
