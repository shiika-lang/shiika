use serde::{Deserialize, Serialize};
use crate::ty::term_ty::TermTy;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct LitTy {
    // REFACTOR: ideally these should be private
    pub base_name: String,
    pub type_args: Vec<TermTy>,
    /// `true` if values of this type are classes 
    pub is_meta: bool,
}

impl LitTy {
    pub fn new(base_name: String, type_args: Vec<TermTy>, is_meta: bool) -> LitTy {
        if base_name == "Metaclass" { debug_assert!(is_meta); }
        LitTy { base_name, type_args, is_meta }
    }
}

