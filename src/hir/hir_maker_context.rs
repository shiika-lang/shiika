use std::collections::HashMap;
use std::rc::Rc;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use crate::hir::*;

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
    /// List of instance variables of the current `self`
    pub ivars: Rc<HashMap<String, SkIVar>>,
    /// Whether we are in an initializer
    pub is_initializer: bool,
}

impl HirMakerContext {
    /// Create a ctx for toplevel
    pub fn toplevel() -> HirMakerContext {
        HirMakerContext {
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: ClassFullname("".to_string()),
            lvars: HashMap::new(),
            ivars: Rc::new(HashMap::new()),
            is_initializer: false,
        }
    }

    /// Create a class context
    pub fn class_ctx(fullname: &ClassFullname) -> HirMakerContext {
        HirMakerContext {
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: fullname.clone(),
            lvars: HashMap::new(),
            ivars: Rc::new(HashMap::new()),
            is_initializer: false,
        }
    }

    /// Create a method context
    pub fn method_ctx(class_ctx: &HirMakerContext, method_sig: &MethodSignature, is_initializer: bool) -> HirMakerContext {
        HirMakerContext {
            method_sig: Some(method_sig.clone()),
            self_ty: ty::raw(&class_ctx.namespace.0),
            namespace: class_ctx.namespace.clone(),
            lvars: HashMap::new(),
            ivars: Rc::clone(&class_ctx.ivars),
            is_initializer: is_initializer,
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
