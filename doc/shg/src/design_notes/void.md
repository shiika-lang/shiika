# Void

`Void` is a singleton type: its sole instance is the `::Void` constant
(initialized right after `Metaclass`, since every Void value loads it). Any
expression of type `Void` evaluates to that one object, so `Void` plays the
role of the unit type.

## Value representation

A Void *value* is a pointer to the `::Void` singleton (a cheap global load, not
an allocation). `codegen`'s `compile_void` loads the constant; the Rust FFI
produces the same singleton (`SkVoid::from(())` returns `::Void`). There is only
one Void object, so equality/identity on Void values is reliable.

## `ret void` for sync Void functions

A function that only ever returns the singleton can instead return nothing. The
`void_ret` MIR pass (run after `asyncness_check`, before `async_splitter`)
lowers a **sync** function whose return type is `Void` to return `CVoid` (LLVM
`void`):

- The function's return type becomes `CVoid` and each `return <v>` becomes
  "evaluate `v`, then `ret void`".
- At a call to such a function the result is `CVoid` (no value); the call is
  wrapped as `Exprs([call, ::Void])` so callers that use the result still get
  the singleton. In a discard position the `::Void` load is unused and is
  removed by LLVM.

The decision is a pure function of `(ret == Void) && sync`, so every
compilation unit — and every call site, local or imported — agrees, keeping the
cross-package ABI consistent (asyncness is part of the exported signature, see
`exports.json5`).

Only **sync** functions are lowered. A method reachable through a vtable is
always async (see "Async runtime" in the architecture chapter), so a sync
method is never virtual: this pass only ever deals with direct calls and never
has to keep a vtable slot's signature consistent across overrides.

### The one exception: `Meta:Void#new`

`Meta:Void#new` returns `Void`, but its result is the singleton object itself —
it is what initializes the `::Void` constant. Its return is a real value, not
unit, so the pass leaves it alone (lowering it would make `::Void` null and
every Void value would dereference null).
