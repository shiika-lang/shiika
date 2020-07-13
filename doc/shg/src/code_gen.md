# CodeGen

Directory: `src/code_gen`

CodeGen generates LLVM IR from Shiika HIR.

## Files

`mod.rs` is the entry point of CodeGen. `gen_exprs.rs` contians the functions of CodeGen which handles Shiika expressions.

## Dependency

Shiika uses `inkwell` crate to generate LLVM IR.
