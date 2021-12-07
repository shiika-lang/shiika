use crate::signature::MethodSignature;
use crate::{HirLVars, HirExpressions};
use shiika_core::{names::*};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SkMethod {
    pub signature: MethodSignature,
    pub body: SkMethodBody,
    pub lvars: HirLVars,
}

pub type SkMethods = HashMap<ClassFullname, Vec<SkMethod>>;

#[derive(Debug)]
pub enum SkMethodBody {
    /// A method defined with Shiika expressions
    Normal { exprs: HirExpressions },
    /// A method defined in skc_rustlib
    RustLib,
    /// The method .new
    New {
        classname: ClassFullname,
        initialize_name: MethodFullname,
        init_cls_name: ClassFullname,
        arity: usize,
        const_is_obj: bool,
    },
    /// A method that just return the value of `idx`th ivar
    Getter { idx: usize, name: String },
    /// A method that just update the value of `idx`th ivar
    Setter { idx: usize, name: String },
}

impl SkMethod {
    /// Returns if this method is defined by skc_rustlib
    pub fn is_rustlib(&self) -> bool {
        matches!(&self.body, SkMethodBody::RustLib)
    }
}
