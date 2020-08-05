use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct CodeGenContext<'run> {
    pub function: inkwell::values::FunctionValue<'run>,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    pub current_loop_end: Option<Rc<inkwell::basic_block::BasicBlock<'run>>>,
    /// Unique id for lambdas
    /// Used for naming their llvm functions
    pub last_lambda_id: usize,
}

impl<'run> CodeGenContext<'run> {
    pub fn new(function: inkwell::values::FunctionValue<'run>) -> CodeGenContext<'run> {
        CodeGenContext {
            function,
            lvars: HashMap::new(),
            current_loop_end: None,
            last_lambda_id: 0,
        }
    }

    /// Return a newly created name for a lambda
    pub fn new_lambda_name(&mut self) -> String {
        self.last_lambda_id += 1;
        format!("lambda_{}", self.last_lambda_id).to_string()
    }
}
