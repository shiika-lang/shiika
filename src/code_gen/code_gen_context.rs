use crate::hir::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct CodeGenContext<'hir: 'run, 'run> {
    /// Current llvm function
    pub function: inkwell::values::FunctionValue<'run>,
    /// If `function` corresponds to a lambda or a method
    pub function_origin: FunctionOrigin,
    /// Parameters of `function`
    /// Only used for lambdas
    pub function_params: Option<&'hir [MethodParam]>,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    /// End of `while`, if any
    pub current_loop_end: Option<Rc<inkwell::basic_block::BasicBlock<'run>>>,
    /// End of the current llvm function. Only used for lambdas
    pub current_func_end: Rc<inkwell::basic_block::BasicBlock<'run>>,
    /// Arguments of `return` found in this context
    pub returns: Vec<(Option<inkwell::values::BasicValueEnum<'run>>, inkwell::basic_block::BasicBlock<'run>)>,
}

#[derive(Debug, PartialEq)]
pub enum FunctionOrigin {
    Method,
    Lambda,
    Other,
}

#[derive(Debug)]
pub struct CodeGenLambda<'hir> {
    pub func_name: String,
    pub params: &'hir [MethodParam],
    pub exprs: &'hir HirExpressions,
}

impl<'hir, 'run> CodeGenContext<'hir, 'run> {
    pub fn new(
        function: inkwell::values::FunctionValue<'run>,
        function_end: Rc<inkwell::basic_block::BasicBlock<'run>>,
        function_origin: FunctionOrigin,
        function_params: Option<&'hir [MethodParam]>,
        lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    ) -> CodeGenContext<'hir, 'run> {
        CodeGenContext {
            function,
            function_origin,
            function_params,
            lvars,
            current_loop_end: None,
            current_func_end: function_end,
            returns: Default::default(),
        }
    }
}
