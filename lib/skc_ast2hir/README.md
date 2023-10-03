# skc\_ast2hir

This crate creates HIR (high-level IR) from AST. Typecheck is also done in this crate.

The HIR itself is defined in another crate, [skc_hir](../skc_hir/).

## Important structs

- `HirMaker` holds all the temporary information needed during compilation.
- `ClassDict` holds information about all the classes (and modules.)
- `HirMakerContext` is stored in `hir_maker.ctx_stack` and holds information
  about the "current" class, method, etc.

## Overview of compilation process

1. `skc_ast2hir::make_hir` is called
1. Create `class_dict::TypeIndex` with `type_index::create`. This is a hashmap from type name to its type parameters.
1. Create `ClassDict` with `class_dict::create`. This collects method signatures in advance of processing the method bodies.
1. Start compilation with `hir_maker::convert_toplevel_items`. This will traverse the entire AST and process each method. Compiled methods are stored as `SkMethod` in `hir_maker.method_dict`.

## Type inferece

Shiika implements two types of type inference:

1. Infer block parameter type

    ```sk
    [1, 2, 3].each{|i: Int| p i}
    can be written as
    [1, 2, 3].each{|i| p i}
    ```

2. Infer method-wise type argument

    ```sk
    [1, 2, 3].map<String>{|i| i.to_s}
    can be written as
    [1, 2, 3].map{|i| i.to_s}

    Also,
    Pair<Int, Bool>.new(1, true)
    can be written as
    Pair.new(1, true)
    which is this form with the tyargs inferred
    Pair.new<Int, Bool>.new(1, true)
    ```

Both are done by per-method-call basis (i.e. not per method definition.)
