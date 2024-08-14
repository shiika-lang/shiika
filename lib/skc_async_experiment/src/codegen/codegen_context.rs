#[derive(Debug)]
pub struct CodeGenContext<'run> {
    /// Current llvm function
    pub function: inkwell::values::FunctionValue<'run>,
}
