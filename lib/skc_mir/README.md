# skc_mir

This crate provides `struct Mir` and `Mir::build`.

## `Mir`

`Hir` represents high-level semantics of a Shiika program but not suitable
for generating LLVM code directly.
`Mir::build` takes a `Hir` and generate various information needed for
LLVM IR generation.

## `mir::VTable`

Vtables are used to implement virtual function call. While languages such as
C++ or C# has the keyword `virtual`, it is not a keyword in Shiika and
all the method calls are invoked via vtable.
