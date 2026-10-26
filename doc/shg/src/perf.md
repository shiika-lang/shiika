# Possible performance improvements

## Unbox Int/Float/Bool

`Int` and `Bool` are now represented as unboxed immediates (`Float` is planned);
see [Unboxing](design_notes/unboxing.md). Arithmetic, comparisons and
conditionals no longer allocate. Values are boxed only where they cross into a
pointer position (containers, captures, `Object`/type-parameter arguments),
which keeps heterogeneous arrays and lambda captures working as before.

## Inline non-capturing blocks

Looping with `each` or `times` are slower than `while` because they involve calling lambdas. However, if the block does not capture any outer variables, it can be inlined to be as fast as `while`.
