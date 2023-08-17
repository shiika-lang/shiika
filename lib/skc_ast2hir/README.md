# skc\_ast2hir

This crate creates HIR (high-level IR) from AST. Typecheck is also done in this crate.

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

## Current algorithm

1. Convert the receiver expr if any, get the `self` if not
1. Resolve method-wise tyargs if explicitly given
1. Lookup the method
  - apply the method\_tyargs if explicitly given
1. Arrange the named arguments
