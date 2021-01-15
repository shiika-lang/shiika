# Possible performance improvements

## Unbox Int/Float/Bool

Currently Int, Float, Bool has their own llvm struct type. 
This may lead to performance penalty (eg. to add to Int's, you need to unbox them, add them and box the result again), but this makes it easier to:

- implement containers.
  - If all shiika values are represented by a pointer (i.e. compatible to `%Object*` via bitcast), heterogenius arrays like `[1, 2, "foo"]` can be built in the same way as building homogenius arrays.
- implement lambda captures.
  - Captured variables need to be boxed.

## Inline non-capturing blocks

Looping with `each` or `times` are slower than `while` because they involve calling lambdas. However, if the block does not capture any outer variables, it can be inlined to be as fast as `while`.
