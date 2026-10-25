# Unboxing of Int / Bool / Float

`Int`, `Bool` and `Float` are represented as **unboxed immediates** — a raw
`i64`, `i1` and `double` respectively — rather than as pointers to heap
objects. This makes the most common operations (arithmetic, comparisons,
conditionals, loop counters) allocation-free.

A boxed form still exists (a heap object with the usual `[vtable, class, value]`
layout) and is used only where a value must look like any other object, i.e.
where it crosses into a *pointer* position. The compiler inserts `box` / `unbox`
coercions at those boundaries.

## Representation

In MIR the immediate types map to the raw LLVM types:

| Shiika | MIR `Ty` | LLVM |
|--------|----------|------|
| `Bool` | `I1`     | `i1` |
| `Int`  | `Int64`  | `i64` |
| `Float`| `F64`    | `double` |

`true` / `false` are plain `i1` constants, integer / float literals are plain
constants, and an `if`/`while` condition is used as a raw `i1` (no unboxing).

The boxed form of an immediate has no distinct MIR type of its own: once it is
boxed it is simply an opaque pointer (the same `ptr` all objects use). So
"boxed Bool" in MIR is just a pointer in an `Object`/type-parameter position.

## Boxing boundaries

The guiding rule is: **a value's representation follows its static type, and the
compiler boxes/unboxes where the static type changes between an immediate and a
pointer position.** An immediate flowing into a pointer position is `box`ed; a
pointer known to hold an immediate flowing back is `unbox`ed.

The ABI of a call is decided by the *callee's declared signature*, not by the
static types of the actual arguments. Concretely, boxing happens at:

- **Method arguments** — an immediate argument passed to a parameter whose
  declared type is erased (`Object`, a type parameter `T`, ...) is boxed.
- **Method results** — when the callee's declared return type is erased (a
  pointer) but this call site expects an immediate, the result is unboxed.
- **Array literals** — elements are stored as pointers, so immediate elements
  are boxed.
- **Lambdas** — see below.
- **Captures** — the captures array holds pointers, so an immediate captured
  variable is boxed on the way in and unboxed when read.
- **Object creation** — a freshly allocated object is always a pointer, so the
  generated `.new` of an immediate class produces a boxed object.
- **`ivar` / constant access** — these load/store using the field's own type,
  so an immediate `ivar`/constant stays unboxed.

Because the HIR already inserts up/down-casts at the `Object`/type-parameter
boundaries (if/case merges, array literals, pattern matching, ...), the
representation change is attached to those cast nodes: a cast whose input is an
immediate and whose target is a pointer performs the box (and vice versa for
unbox). This keeps the boxing logic in one place and hard to forget.

## Generics and lambdas

Type parameters are erased, so generic code works on pointers. A lambda is
invoked through the generic `FnN#call`, whose parameter and return types are
`FnN`'s type parameters — always erased. The lambda function therefore uses a
**fully erased ABI**: it receives immediate parameters boxed (and unboxes them
on use) and boxes an immediate result before returning. Invocation sites box
immediate arguments and unbox an immediate result accordingly.

## Method dispatch

`Int`/`Bool`/`Float` are final, so a call whose *static receiver type* is an
immediate is always a **direct call** with the raw receiver — it is never routed
through the vtable (a vtable lookup would read an object header from what is
really an `i1`/`i64`). This keeps arithmetic and comparisons as plain machine
instructions.

A boxed immediate (one that has escaped into an `Object`-typed variable) can
still receive a *virtual* call. For that case each virtual method of an
immediate class has a **boxed-receiver wrapper** registered in the class's
vtable slot: the wrapper unboxes the receiver (and any immediate arguments),
calls the raw method, and returns. So an immediate method has two entry points —
the raw one for direct calls (fast path) and the wrapper for vtable dispatch
(cold path).

## Async / CPS

The async transformation stores live values in the `ChiikaEnv` as an opaque
`i64` ("Any"). Immediates round-trip through it without boxing: `i1` is
zero-extended / truncated and `f64` is bit-cast to and from `i64`.

## FFI / corelib

A corelib method written in Rust receives and returns immediates by value:
`SkInt` is a transparent `i64`, `SkBool` a transparent `bool` (C `i8`, bridged
to LLVM `i1`), and `SkFloat` an `f64`. There is no boxed struct to dereference.

## Default arguments

A method with default parameters keeps its immediate parameters unboxed and uses
a presence bitmask to detect omitted arguments; see [MIR](../mir.md).

## Current status

- `Bool` and `Int` are unboxed. `Float` representation (`F64`) exists and the
  boundary machinery handles it, but `from_term_ty` does not yet map `Float` to
  `F64` (Phase 3).
- The boxed `Bool` is not yet a singleton, so each boxing allocates and
  `equal?` on boxed `Bool`s is not reliable.
