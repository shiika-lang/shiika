use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct CodeGenContext<'codegen> {
    pub function: inkwell::values::FunctionValue<'codegen>,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue<'codegen>>,
    pub current_loop_end: Option<Rc<inkwell::basic_block::BasicBlock<'codegen>>>,
}

impl<'codegen> CodeGenContext<'codegen> {
    pub fn new(function: inkwell::values::FunctionValue) -> CodeGenContext<'codegen> {
        CodeGenContext {
            function,
            lvars: HashMap::new(),
            current_loop_end: None,
        }
    }
}
