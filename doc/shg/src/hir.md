# HIR

Directory: `src/hir/`

AST is converted into HIR (High-level Intermediate Representation) and then turns into LLVM IR (Low-level IR of Shiika).

The structure of HIR resembles to AST, but most important difference is that HIR has type information.

## mod.rs

File: `src/hir/mod.rs`

This file contains the structure of HIR.

Structure of types are defined in another file, `src/ty.rs`.

## HirMaker

File: `src/hir/hir_maker.rs`, `convert_exprs.rs`

These two files contains the main process of converting AST into HIR.

