use crate::hir::*;
use crate::ty::*;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Debug)]
pub struct CodeGenContext<'hir: 'run, 'run> {
    /// Current llvm function
    pub function: inkwell::values::FunctionValue<'run>,
    /// If `function` corresponds to a lambda or a method
    /// (llvm func of methods takes `self` as the first arg but lambdas do not)
    pub function_origin: FunctionOrigin,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    pub current_loop_end: Option<Rc<inkwell::basic_block::BasicBlock<'run>>>,
    /// Unique id for lambdas
    /// Used for naming their llvm functions
    pub last_lambda_id: usize,
    /// Lambdas to be compiled
    pub lambdas: VecDeque<CodeGenLambda<'hir>>,
}

#[derive(Debug)]
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
        function_origin: FunctionOrigin,
    ) -> CodeGenContext<'hir, 'run> {
        CodeGenContext {
            function,
            function_origin,
            lvars: HashMap::new(),
            current_loop_end: None,
            last_lambda_id: 0,
            lambdas: VecDeque::new(),
        }
    }

    /// Return a newly created name for a lambda
    pub fn new_lambda_name(&mut self) -> String {
        self.last_lambda_id += 1;
        format!("lambda_{}", self.last_lambda_id).to_string()
    }

    /// Push a lambda into the queue
    pub fn push_lambda(
        &mut self,
        func_name: String,
        params: &'hir [MethodParam],
        exprs: &'hir HirExpressions,
    ) {
        let l = CodeGenLambda {
            func_name,
            params,
            exprs,
        };
        self.lambdas.push_back(l);
    }
}
