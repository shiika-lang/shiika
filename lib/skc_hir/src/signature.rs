use serde::{Deserialize, Serialize};
use shiika_core::{names::*, ty, ty::*};
use std::collections::HashMap;

pub type MethodSignatures = HashMap<MethodFirstname, MethodSignature>;

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
    pub fn specialize(&self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> MethodSignature {
        MethodSignature {
            fullname: self.fullname.clone(),
            ret_ty: self.ret_ty.substitute(class_tyargs, method_tyargs),
            params: self
                .params
                .iter()
                .map(|param| param.substitute(class_tyargs, method_tyargs))
                .collect(),
            typarams: self.typarams.clone(), // eg. Array<T>#map<U>(f: Fn1<T, U>) -> Array<Int>#map<U>(f: Fn1<Int, U>)
        }
    }

    /// Returns true if `self` is the same as `other` except the
    /// parameter names.
    pub fn equivalent_to(&self, other: &MethodSignature) -> bool {
        if self.fullname.first_name != other.fullname.first_name {
            return false;
        }
        if !self.ret_ty.equals_to(&other.ret_ty) {
            return false;
        }
        if self.params.len() != other.params.len() {
            return false;
        }
        for i in 0..self.params.len() {
            if self.params[i].ty != other.params[i].ty {
                return false;
            }
        }
        if self.typarams != other.typarams {
            return false;
        }
        return true;
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct MethodParam {
    pub name: String,
    pub ty: TermTy,
}

impl MethodParam {
    pub fn substitute(&self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> MethodParam {
        MethodParam {
            name: self.name.clone(),
            ty: self.ty.substitute(class_tyargs, method_tyargs),
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
    metaclass_fullname: &ClassFullname,
    initialize_params: Vec<MethodParam>,
    instance_ty: &TermTy,
) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(metaclass_fullname, "new"),
        ret_ty: instance_ty.clone(),
        params: initialize_params,
        typarams: vec![],
    }
}

/// Create a signature of a `initialize` method
pub fn signature_of_initialize(
    class_fullname: &ClassFullname,
    params: Vec<MethodParam>,
) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(class_fullname, "initialize"),
        ret_ty: ty::raw("Void"),
        params,
        typarams: vec![],
    }
}
