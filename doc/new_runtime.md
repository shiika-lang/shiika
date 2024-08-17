# New runtime for concurrency

Tracking issue: https://github.com/shiika-lang/shiika/issues/545

## Files

- src/bin/exp_shiika.rs
- lib/skc_runtime/
- lib/skc_async_experiment/

## How to try

1. Run `cargo build` at lib/skc_runtime
2. `cargo run --bin exp_shiika -- a.milika"`
3. `./a`

## Syntax

Currently the syntax and semantics are the same as that of [milika](https://github.com/yhara/milika). Example:

```
extern print(Int n) -> Null
extern(async) sleep_sec(Int n) -> Null
fun chiika_main() -> Int {
  print(123)
  return 0
}
```
