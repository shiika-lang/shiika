use shiika_core::ty::TermTy;
use skc_hir::{MethodSignature, SkType};

#[derive(Debug, Clone)]
pub struct FoundMethod<'hir_maker> {
    /// A Shiika class or Shiika module
    pub owner: &'hir_maker SkType,
    /// The signature of the method
    pub sig: MethodSignature,
    /// Index in the method list of `owner` (used for module method call via wtable)
    pub method_idx: Option<usize>,
}

impl<'hir_maker> FoundMethod<'hir_maker> {
    pub fn class(owner: &'hir_maker SkType, sig: MethodSignature) -> FoundMethod<'hir_maker> {
        debug_assert!(owner.is_class());
        FoundMethod {
            owner,
            sig,
            method_idx: None,
        }
    }

    pub fn module(
        owner: &'hir_maker SkType,
        sig: MethodSignature,
        idx: usize,
    ) -> FoundMethod<'hir_maker> {
        debug_assert!(!owner.is_class());
        FoundMethod {
            owner,
            sig,
            method_idx: Some(idx),
        }
    }

    pub fn specialize(&mut self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) {
        self.sig = self.sig.specialize(class_tyargs, method_tyargs);
    }

    pub fn set_class(&self, owner: &'hir_maker SkType) -> FoundMethod<'hir_maker> {
        debug_assert!(owner.is_class());
        FoundMethod {
            owner,
            ..self.clone()
        }
    }

    pub fn set_module(&self, owner: &'hir_maker SkType) -> FoundMethod<'hir_maker> {
        debug_assert!(!owner.is_class());
        FoundMethod {
            owner,
            ..self.clone()
        }
    }
}
