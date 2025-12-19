use anyhow::{anyhow, Result};
use inkwell::values::CallSiteValue;

/// Check errors LLVM IR that are not well handled in LLVM (i.e., LLVM will crash but not report
/// where it happened).
pub fn run(module: &inkwell::module::Module) -> Result<()> {
    let mut errors: Vec<String> = vec![];
    for function in module.get_functions() {
        let fname = format!("{:?}", function.get_name());
        for basic_block in function.get_basic_blocks() {
            for instruction in basic_block.get_instructions() {
                if let Ok(call_site) = CallSiteValue::try_from(instruction) {
                    if let Some(callee) = call_site.get_called_fn_value() {
                        let callee_arity = callee.count_params();
                        let call_arity = call_site.count_arguments();
                        if callee_arity != call_arity {
                            errors.push(format!(
                                "- Arity mismatch in call to {:?} in {fname}\n  params: {}\n  args: {}",
                                callee.get_name(),
                                callee_arity,
                                call_arity
                            ));
                        }
                        for i in 0..call_site.count_arguments() {
                            if let Some(arg) = instruction.get_operand(i) {
                                if arg.is_value() {
                                    let arg_value = arg.unwrap_value();
                                    let arg_type = arg_value.get_type();
                                    if let Some(param_value) = callee.get_nth_param(i) {
                                        let param_type = param_value.get_type();
                                        if arg_type != param_type {
                                            errors.push(format!("- Type mismatch in call to {:?} in {fname}\n  arg {i}: {:?}\n  param {i}: {:?}",  
                                                                callee.get_name(), arg_type, param_type));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("invalid LLVM IR: \n{}", errors.join("\n")))
    }
}
