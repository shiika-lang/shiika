/// Provides utility functions used by code_gen/*.rs
use crate::code_gen::*;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Return the llvm func
    /// Panic if not found
    pub (in super) fn get_llvm_func(
        &self,
        name: &str
    ) -> inkwell::values::FunctionValue<'ictx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| {
                panic!(
                    "[BUG] get_llvm_func: `{:?}' not found",
                    name
                )
            })
    }
}
