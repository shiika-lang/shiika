//! This module defines dummy structs to express dependencies hidden in the LLVM layer.

/// LLVM Functions compiled from Shiika methods
#[derive(Debug, Clone, Copy)]
pub struct MethodFuncs();

/// LLVM Global Constants for Shiika constants
#[derive(Debug, Clone, Copy)]
pub struct ConstGlobal();
