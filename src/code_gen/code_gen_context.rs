use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct CodeGenContext {
    pub function: inkwell::values::FunctionValue,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue>,
    pub current_loop_end: Option<Rc<inkwell::basic_block::BasicBlock>>,
}

impl CodeGenContext {
    pub fn new(function: inkwell::values::FunctionValue) -> CodeGenContext {
        CodeGenContext {
            function: function,
            lvars: HashMap::new(),
            current_loop_end: None,
        }
    }
}
