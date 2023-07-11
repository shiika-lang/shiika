use shiika_core::ty::{Erasure, TermTy};
use skc_hir::{MethodSignature, SkType};

#[derive(Debug, Clone)]
pub struct FoundMethod {
    /// A Shiika class or Shiika module
    pub owner: Erasure,
    /// The signature of the method
    pub sig: MethodSignature,
    /// Index in the method list of `owner` (used for module method call via wtable)
    pub method_idx: Option<usize>,
}

impl FoundMethod {
    pub fn class(owner: &SkType, sig: MethodSignature) -> FoundMethod {
        debug_assert!(owner.is_class());
        FoundMethod {
            owner: owner.erasure().clone(),
            sig,
            method_idx: None,
        }
    }

    pub fn module(owner: &SkType, sig: MethodSignature, idx: usize) -> FoundMethod {
        debug_assert!(!owner.is_class());
        FoundMethod {
            owner: owner.erasure().clone(),
            sig,
            method_idx: Some(idx),
        }
    }

    pub fn specialize(&mut self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) {
        self.sig = self.sig.specialize(class_tyargs, method_tyargs);
    }

    pub fn set_class(&self, owner: &SkType) -> FoundMethod {
        debug_assert!(owner.is_class());
        FoundMethod {
            owner: owner.erasure().clone(),
            ..self.clone()
        }
    }

    pub fn set_module(&self, owner: &SkType) -> FoundMethod {
        debug_assert!(!owner.is_class());
        FoundMethod {
            owner: owner.erasure().clone(),
            ..self.clone()
        }
    }

    /// Returns if this is of the form `Foo.new<Bar>`
    pub fn is_generic_new(&self, receiver_ty: &TermTy) -> bool {
        self.sig.fullname.first_name.0 == "new"
            && receiver_ty.is_metaclass()
            && !receiver_ty.has_type_args()
            && !self.sig.typarams.is_empty()
    }
}
