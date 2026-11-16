# Tasks

This note describes how `Task.scope` and `Task::Scope#spawn` are implemented.
See the Programming Guide for the usage.

## Execution model

- The runtime is a single-threaded tokio runtime (`chiika_start_tokio` in
  `packages/core/ext/src/runtime.rs`). Shiika code is never run in parallel.
- Each task has its own `ChiikaEnv`, i.e. its own Shiika stack. Tasks share
  data only via objects on the GC heap (eg. variables captured by the block).

## No compiler support

`spawn` is an ordinary method; the compiler does not know about tasks.

- A block is compiled to a `Fn0` object. As lambdas are always compiled as
  async functions, its `@func` has the lowered signature
  `(env, fn_obj, cont) -> ContFuture`. The runtime calls it on a new tokio task
  with a new `ChiikaEnv` and `chiika_finish` as the continuation
  (`packages/core/ext/src/runtime/task.rs`).
- `tokio::spawn` is a sync function, so `spawn` itself is a sync method.
- `return` in a block is rejected for any block (#266), and `break` is
  rejected because `Task.scope` and `spawn` return a value. So a block of a
  task never tries to exit to the outer method, which would be impossible as
  the task has a different stack.

## Where the states live

Rust values which own resources (`Notify`, etc.) must not be stored in Shiika
objects, because Shiika objects are allocated by GC and never dropped.
So the states are kept in a table on the Rust side, and `Task::Scope` and
`Task` only have an id.

- Scope: the number of running tasks and a `Notify` to wait for them.
  `Task.scope` waits until the number gets zero, then removes the entry
  (the scope is "closed").
- Task: a `Notify` to wait for the task. The entry is removed when the task
  finishes.

The table is only for waiting. The value of a task is stored in the `Task`
object by Shiika code: `spawn` wraps the given block with a lambda which calls
`Task#_finish` with the value. This way the Rust side does not need to know the
layout of `Task` nor to keep a Shiika value in the table, and a `Task` can be
awaited after its entry is removed.

## Panics

A panic in Shiika is not meant to be caught. However tokio catches a panic in
a task and just records it in the `JoinHandle`, so the runtime explicitly
terminates the process when a task panics.
