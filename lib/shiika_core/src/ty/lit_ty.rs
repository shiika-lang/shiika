use crate::ty::term_ty::TermTy;
use crate::ty;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct LitTy {
    // REFACTOR: ideally these should be private
    pub base_name: String,
    pub type_args: Vec<TermTy>,
    /// `true` if values of this type are classes
    pub is_meta: bool,
}

impl LitTy {
    pub fn new(base_name: String, type_args: Vec<TermTy>, is_meta_: bool) -> LitTy {
        let is_meta = if base_name == "Metaclass" {
            // There is no `Meta:Metaclass`
            true
        } else {
            is_meta_
        };
        LitTy {
            base_name,
            type_args,
            is_meta,
        }
    }

    pub fn raw(base_name: &str) -> LitTy {
        LitTy::new(base_name.to_string(), vec![], false)
    }

    pub fn to_term_ty(&self) -> TermTy {
        ty::new(self.base_name.clone(), self.type_args.clone(), self.is_meta)
    }

    pub fn into_term_ty(self) -> TermTy {
        ty::new(self.base_name, self.type_args, self.is_meta)
    }
}
