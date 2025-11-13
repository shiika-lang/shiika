use crate::method_signature::MethodSignature;
use crate::{HirExpression, HirLVars};
use shiika_core::names::*;
use shiika_core::ty::TermTy;
use std::collections::HashMap;

#[derive(Debug)]
pub struct SkMethod {
    pub fullname: MethodFullname,
    pub body: SkMethodBody,
    pub lvars: HirLVars,
}

pub type SkMethods = HashMap<TypeFullname, Vec<SkMethod>>;

#[derive(Debug)]
pub enum SkMethodBody {
    /// A method defined with Shiika expressions
    Normal { exprs: HirExpression },
    /// A method defined in skc_rustlib
    RustLib,
    /// The method .new
    New {
        classname: ClassFullname,
        initializer: Option<MethodSignature>,
        arity: usize,
        const_is_obj: bool,
    },
    /// A method that just return the value of `idx`th ivar
    Getter {
        idx: usize,
        name: String,
        ty: TermTy,
        self_ty: TermTy,
    },
    /// A method that just update the value of `idx`th ivar
    Setter {
        idx: usize,
        name: String,
        ty: TermTy,
        self_ty: TermTy,
    },
}

impl SkMethod {
    /// Create a SkMethod which does not use lvar at all.
    pub fn simple(fullname: MethodFullname, body: SkMethodBody) -> SkMethod {
        SkMethod {
            fullname,
            body,
            lvars: Default::default(),
        }
    }

    /// Returns if this method is defined by skc_rustlib
    pub fn is_rustlib(&self) -> bool {
        matches!(&self.body, SkMethodBody::RustLib)
    }
}
