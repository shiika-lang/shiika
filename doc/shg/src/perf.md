# Possible performance improvements

## Unbox Int/Float/Bool

`Int`, `Bool` and `Float` are now represented as unboxed immediates;
see [Unboxing](design_notes/unboxing.md). Arithmetic, comparisons and
conditionals no longer allocate. Values are boxed only where they cross into a
pointer position (containers, captures, `Object`/type-parameter arguments),
which keeps heterogeneous arrays and lambda captures working as before.

## Primitive comparisons

Concrete Int/Float comparisons (`<`, `<=`, `>`, `>=`, `==`) and Bool equality
become MIR `PrimBinOp` nodes and LLVM `icmp` / ordered `fcmp` instructions. Removing the method
call before asyncness inference also removes the comparison's CPS dependency.
Comparison calls through wtables retain their async implementations.
See [Method dispatch](design_notes/unboxing.md#method-dispatch).

For `examples/fib.sk` (`fib(30)`), the generated `A#fib` changes from async to
sync. A local Linux aarch64 measurement with the default debug compiler/core
build, unchanged linker options and three executions of each binary gave:

| | Before | After |
|---|---:|---:|
| Median elapsed time | 1.349 s | 0.025 s |
| Range | 1.326–1.362 s | 0.023–0.033 s |

These are whole-process times, including runtime startup and output. Both
binaries print `832040`; the generated IR also confirms the direct comparison
call has been replaced by `icmp slt`. These measurements preceded the addition
of Float comparison intrinsics.

With the same build settings, `examples/lifegame.sk` took a median 0.353 s
before and 0.316 s after (three runs each; ranges 0.346–0.355 s and
0.314–0.328 s). Every run matched `lifegame.expected_out.txt` exactly. This
example still spends time in blocks, indirect calls and output.

## Inline non-capturing blocks

Looping with `each` or `times` are slower than `while` because they involve calling lambdas. However, if the block does not capture any outer variables, it can be inlined to be as fast as `while`.
