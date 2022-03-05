use serde::{Deserialize, Serialize};
use shiika_core::{names::*, ty, ty::*};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct MethodSignature {
    pub fullname: MethodFullname,
    pub ret_ty: TermTy,
    pub params: Vec<MethodParam>,
    pub typarams: Vec<TyParam>,
}

impl MethodSignature {
    pub fn first_name(&self) -> &MethodFirstname {
        &self.fullname.first_name
    }

    /// Substitute type parameters with type arguments
    pub fn specialize(&self, module_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> MethodSignature {
        MethodSignature {
            fullname: self.fullname.clone(),
            ret_ty: self.ret_ty.substitute(module_tyargs, method_tyargs),
            params: self
                .params
                .iter()
                .map(|param| param.substitute(module_tyargs, method_tyargs))
                .collect(),
            typarams: self.typarams.clone(), // eg. Array<T>#map<U>(f: Fn1<T, U>) -> Array<Int>#map<U>(f: Fn1<Int, U>)
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct MethodParam {
    pub name: String,
    pub ty: TermTy,
}

impl MethodParam {
    pub fn substitute(&self, module_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> MethodParam {
        MethodParam {
            name: self.name.clone(),
            ty: self.ty.substitute(module_tyargs, method_tyargs),
        }
    }
}

/// Return a param of the given name and its index
pub fn find_param<'a>(params: &'a [MethodParam], name: &str) -> Option<(usize, &'a MethodParam)> {
    params
        .iter()
        .enumerate()
        .find(|(_, param)| param.name == name)
}

/// Create a signature of a `new` method
pub fn signature_of_new(
    metamodule_fullname: &ModuleFullname,
    initialize_params: Vec<MethodParam>,
    instance_ty: &TermTy,
) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(metamodule_fullname, "new"),
        ret_ty: instance_ty.clone(),
        params: initialize_params,
        typarams: vec![],
    }
}

/// Create a signature of a `initialize` method
pub fn signature_of_initialize(
    module_fullname: &ModuleFullname,
    params: Vec<MethodParam>,
) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(module_fullname, "initialize"),
        ret_ty: ty::raw("Void"),
        params,
        typarams: vec![],
    }
}
