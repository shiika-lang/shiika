# MIR

Directory: `lib/skc_main/src/mir/`, `lib/skc_main/src/mirgen/`,
`lib/skc_main/src/mir_lowering/`

MIR (Mid-level Intermediate Representation) sits between HIR and LLVM
IR. While HIR is still close to the source program (classes, methods,
Shiika expressions), MIR is function-based: methods and lambdas are
lowered into plain functions (`mir::Function`), and the async
transformation (CPS conversion) is performed on MIR.

Note: the crate `lib/skc_mir` is _not_ the home of this MIR; it was
made for the old runtime and now mainly provides vtable building
(`VTables`) and library export structures.

## Structures

Defined in `lib/skc_main/src/mir.rs`:

- `CompilationUnit`: everything codegen needs — the `Program` plus
  `sk_types`, `vtables` and the imported (from dependency packages)
  counterparts.
- `Program`: `classes`, `externs` (functions defined elsewhere,
  e.g. in Rust), `funcs` and `constants`.
- `mir::Expr` (`mir/expr.rs`): MIR-level expressions.
- `mir/visitor.rs`, `mir/rewriter.rs`: traversal helpers used by the
  lowering passes.
- `mir/verifier.rs`: type-checks the final MIR before codegen.

## MirGen

Directory: `lib/skc_main/src/mirgen/`

Converts HIR into MIR. Notable parts:

- `pattern_match.rs`: compiles `match` clauses into nested `if`s.
- `lambda.rs`: lowers fns/blocks into functions.
- `prepare_asyncness.rs`: initial asyncness bookkeeping.

## Lowering passes

Directory: `lib/skc_main/src/mir_lowering/`

After MirGen, the passes below run in this order (see
`lib/skc_main/src/build/compiler.rs`). Each pass dumps the MIR to
`$SHIIKA_WORK/debug_logs/NN-<pass>.mirdump` when debug logging is
enabled.

1. `lower_create_type_object`: lower `CreateTypeObject` into
   `Meta:Class#_new` calls.
2. `asyncness_check`: determine each function's asyncness
   (a function calling an async function is itself async;
   see [Architecture](architecture.md)).
3. `splice_exprs`: flatten nested `Exprs` nodes.
4. `let_bind_async`: bind results of nested async calls to
   temporary variables so that every async call appears in a
   statement position.
5. `pass_async_env`: add the `env` parameter to async functions and
   convert their local variables into env accesses.
6. `async_splitter`: split each async function at its async call
   sites into multiple CPS functions (the heart of the async
   transformation).
7. `lower_vtable_ref`: lower `VTableRef` into `GetVTable` + `FunCall`.
8. `resolve_env_op`: convert `EnvGet`/`EnvSet` into function calls.
9. `insert_allocs`: insert an `Alloc` for each `LVarDecl`.

Finally `mir::verifier` checks the result and codegen turns it into
LLVM IR.
