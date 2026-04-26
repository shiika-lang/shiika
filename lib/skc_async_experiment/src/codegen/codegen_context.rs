use std::collections::HashMap;

#[derive(Debug)]
pub struct CodeGenContext<'run> {
    /// Current llvm function
    pub function: inkwell::values::FunctionValue<'run>,
    pub lvars: HashMap<String, inkwell::values::PointerValue<'run>>,
    /// Stack of `WhileEnd` blocks for the enclosing `while` loops; used by `break`.
    pub while_end_stack: Vec<inkwell::basic_block::BasicBlock<'run>>,
}
