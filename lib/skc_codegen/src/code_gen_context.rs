use crate::values::SkObj;
use skc_hir::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct CodeGenContext<'hir: 'run, 'run> {
    /// Current llvm function
    pub function: inkwell::values::FunctionValue<'run>,
    /// If `function` corresponds to a lambda or a method
    pub function_origin: FunctionOrigin<'hir>,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    /// End of `while`, if any
    pub current_loop_end: Option<Rc<inkwell::basic_block::BasicBlock<'run>>>,
    /// End of the current llvm function. Only used for lambdas
    pub current_func_end: Rc<inkwell::basic_block::BasicBlock<'run>>,
    /// Arguments of `return` found in this context
    pub returns: Vec<(SkObj<'run>, inkwell::basic_block::BasicBlock<'run>)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FunctionOrigin<'hir> {
    Method {
        params: &'hir [MethodParam],
    },
    Lambda {
        name: String,
        params: &'hir [MethodParam],
    },
    Other,
}

impl<'hir, 'run> CodeGenContext<'hir, 'run> {
    pub fn new(
        function: inkwell::values::FunctionValue<'run>,
        function_end: Rc<inkwell::basic_block::BasicBlock<'run>>,
        function_origin: FunctionOrigin<'hir>,
        lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    ) -> CodeGenContext<'hir, 'run> {
        CodeGenContext {
            function,
            function_origin,
            lvars,
            current_loop_end: None,
            current_func_end: function_end,
            returns: Default::default(),
        }
    }

    /// Inject `lvars` to `self.lvars`
    /// Returns the original HashMap.
    pub fn inject_lvars(
        &mut self,
        lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    ) -> HashMap<String, inkwell::values::PointerValue<'run>> {
        let mut new_lvars = self
            .lvars
            .clone()
            .into_iter()
            .chain(lvars.into_iter())
            .collect();
        std::mem::swap(&mut new_lvars, &mut self.lvars);
        new_lvars
    }

    pub fn lambda_name(&self) -> Option<&str> {
        match &self.function_origin {
            FunctionOrigin::Lambda { name, .. } => Some(name),
            _ => None,
        }
    }
}
