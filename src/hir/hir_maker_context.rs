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
        }
    }
}
