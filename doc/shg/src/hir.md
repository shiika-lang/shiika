# HIR

Directory: `lib/skc_hir`

AST is converted into HIR (High-level Intermediate Representation) and then turns into LLVM IR (Low-level IR of Shiika).

The structure of HIR resembles to AST, but most important difference is that HIR has type information.

## HirMaker

File: `skc_ast2hir`

This crate converts AST into HIR.

## Lambda

Instances of `Fn` are called "lambda" in Shiika. There are two ways to create a lambda:

1. `fn(){ ...}` (called "fn")
2. `do ... end`, `{ ... }` (called "block")

A lambda can capture outer variables.

```
var a = 1
f = fn(){ p a }
a = 2
f() #=> prints `2`
```

`HirLambdaExpr` contains `captures`, a list of `HirLambdaCapture`. To make this:

1. When referring/updating a local variable defined in outer scope, save a `LambdaCapture` to `captures` of `LambdaCtx`.
2. Once all exprs in a lambda are processed, convert each `LambdaCapture` to `HirLambdaCapture`.
