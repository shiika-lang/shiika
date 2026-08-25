# Architecture

Shiika is a compiler written in Rust that compiles to LLVM IR.

## Compilation pipeline

```
.sk files → AST → HIR → MIR → LLVM IR → Executable
```

## Crate layout

- entry point: `src/main.rs`
- Parsing
  - `lib/shiika_parser`: hand-written parser converting source to AST
  - `lib/shiika_ast`: AST structures, tokens, and visitor patterns
- HIR generation
  - `lib/skc_main/hir/`
  - `lib/skc_ast2hir/`: `ClassDict` and related helpers shared by HIR generation
  - `lib/skc_hir/`: HIR structures (`SkMethod`, `SkType`, `SkClass`)
- MIR generation
  - `lib/skc_main/hir_to_mir.rs`
  - `lib/skc_mir/`: vtable building and library export structures
    (This crate is made for the old runtime. Should be renamed)
- Code generation
  - `lib/skc_main/codegen/`
- Runtime and stdlib
  - `packages/core`
- FFI
  - `lib/shiika_ffi/core_class`
  - Use `shiika_ffi` and `shiika_ffi_macro` for integrating external Rust code.

`lib/shiika_core` defines types and names shared across the crates.

The compiler uses `anyhow::Result` for error handling and `ariadne` for
pretty error reporting.

## Async runtime

Concurrency is implemented with a Continuation-Passing Style (CPS)
transformation on top of a Tokio-based async runtime.

Methods and functions has no "color" in Shiika level. However, internally
- those defined in Rust is marked async or sync (in `exports.json5`), and
- those calling async methods are inferred transitively as async.

## Virtual methods

### VTable (Virtual Method Table)

VTables implements inheritance.

- **Structure built**: MIR stage (`lib/skc_mir/src/vtables.rs`, `VTables::build()`).
  Inherits the superclass vtable and adds/overrides methods.
- **LLVM insertion**: codegen stage (`lib/skc_main/src/codegen/`).
  Creates global constants like `@shiika_vtable_<ClassName>`.

### WTable (Witness Table)

WTables implement module inclusion (similar to Rust's trait objects). A WTable
maps module methods to actual implementations.

- **Structure built**: HIR stage
  (`lib/skc_ast2hir/src/class_dict/build_wtable.rs`, `build_wtable()`).
  For example, for `Array` including `Enumerable`, it maps
  `Enumerable#each` → `Array#each`.
