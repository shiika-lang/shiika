# Tasks

A Shiika program can run multiple tasks concurrently.

## Starting tasks

Tasks are started with `Task.scope` and `spawn`.

```sk
Task.scope do |scope|
  scope.spawn{ download("https://example.com/a") }
  scope.spawn{ download("https://example.com/b") }
end
# Both downloads are done here.
```

`Task.scope` runs the block and then waits for all the tasks spawned in the
scope. In other words, a task never outlives the scope it belongs to.
`Task.scope` returns the value of the block.

A task can spawn another task in the same scope, even after the block of
`Task.scope` has finished. Calling `spawn` after all the tasks of the scope
have finished is an error (panic).

## Getting the result

`spawn` returns a `Task<T>`, where `T` is the type of the block. `Task#await`
waits for the task and returns the value.

```sk
Task.scope do |scope|
  let a = scope.spawn{ download("https://example.com/a") }
  let b = scope.spawn{ download("https://example.com/b") }
  puts a.await + b.await
end
```

You do not have to call `await`; `Task.scope` waits for the task anyway.

## Tasks are not run in parallel

Only one task runs at a time. A task is switched to another only while it is
waiting for something (I/O, `sleep_sec`, `Task#await`, etc.)

- You do not need locks to protect data shared by tasks. Just note that
  other tasks may run during a call which waits.
- A task which does a long computation without waiting keeps the other tasks
  from running. Call `Task.yield` once in a while to let them run.

```sk
scope.spawn do
  items.each do |item|
    heavy_computation(item)
    Task.yield
  end
end
```

## Restrictions

- `return` and `break` are not allowed in the blocks of `Task.scope` and
  `spawn`.
- A panic in a task is not caught; it terminates the program. If a task may
  fail, return a `Result` from the block.
- A task cannot be canceled (yet).
