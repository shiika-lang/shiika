use std::collections::HashMap;

#[derive(Debug)]
pub struct CodeGenContext {
    pub function: inkwell::values::FunctionValue,
    /// Ptr of local variables
    pub lvars: HashMap<String, inkwell::values::PointerValue>
}

impl CodeGenContext {
    pub fn new(function: inkwell::values::FunctionValue) -> CodeGenContext {
        CodeGenContext {
            function: function,
            lvars: HashMap::new(),
        }
    }
}
