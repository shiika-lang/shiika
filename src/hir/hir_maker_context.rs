use crate::names::*;
use crate::ty;
use crate::ty::*;

#[derive(Debug)]
pub struct HirMakerContext {
//    /// Local variables of the current function found so far
//    local_vars: HashSet<CtxLVar>,
    /// Signature of the current method
    /// (Used to get the list of parameters)
    ///
    /// On the toplevel, this will be a dummy signature with no params. 
    pub method_sig: MethodSignature,
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
        let dummy_sig = MethodSignature {
            name: MethodName("(dummy)".to_string()),
            fullname: MethodFullname { full_name: "(dummy)".to_string(), first_name: "(dummy)".to_string() },
            ret_ty: ty::raw("Void"),
            params: vec![],
        };

        HirMakerContext {
            method_sig: dummy_sig,
            self_ty: ty::raw("Object"),
        }
    }
}
