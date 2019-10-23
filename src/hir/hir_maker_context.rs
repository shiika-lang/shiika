use crate::names::*;
use crate::ty;
use crate::ty::*;

#[derive(Debug)]
pub struct HirMakerContext {
//    /// Local variables of the current function found so far
//    local_vars: HashSet<CtxLVar>,
    /// Signature of the current method (Used to get the list of parameters)
    /// None if out of a method
    pub method_sig: Option<MethodSignature>,
    /// The type of current `self`
    pub self_ty: TermTy,
    /// Current namespace
    /// `""` for toplevel
    pub namespace: ClassFullname
//    // List of instance variables of the current `self`
//    //self_ivars: HashMap<IVarName, TermTy>,
}

//pub struct CtxLVar {
//    name: LVarName,
//    ty: TermTy
//}

impl HirMakerContext {
    /// Create a ctx for toplevel
    pub fn toplevel() -> HirMakerContext {
        HirMakerContext {
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: ClassFullname("".to_string()),
        }
    }

    /// Create a class context
    pub fn class_ctx(fullname: &ClassFullname) -> HirMakerContext {
        HirMakerContext {
            method_sig: None,
            self_ty: ty::raw("Object"),
            namespace: fullname.clone(),
        }
    }

    /// Create a method context
    pub fn method_ctx(class_ctx: &HirMakerContext, method_sig: &MethodSignature) -> HirMakerContext {
        HirMakerContext {
            method_sig: Some(method_sig.clone()),
            self_ty: ty::raw(&class_ctx.namespace.0),
            namespace: class_ctx.namespace.clone(),
        }
    }
}
