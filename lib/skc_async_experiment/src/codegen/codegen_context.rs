use std::collections::HashMap;

#[derive(Debug)]
pub struct CodeGenContext<'run> {
    /// Current llvm function
    pub function: inkwell::values::FunctionValue<'run>,
    pub lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
}
