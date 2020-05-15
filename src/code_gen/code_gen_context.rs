use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct CodeGenContext<'run> {
    pub function: inkwell::values::FunctionValue<'run>,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    pub current_loop_end: Option<Rc<inkwell::basic_block::BasicBlock<'run>>>,
}

impl<'run> CodeGenContext<'run> {
    pub fn new(function: inkwell::values::FunctionValue<'run>) -> CodeGenContext<'run> {
        CodeGenContext {
            function,
            lvars: HashMap::new(),
            current_loop_end: None,
        }
    }
}
