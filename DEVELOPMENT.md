# Development Guide

This document describes how to build, test, and debug the Shiika compiler.

For the internal structure of the compiler, see the
[Shiika Hacking Guide](./doc/shg/src/SUMMARY.md).

## Environment variables

- `$SHIIKA_ROOT`: the directory where the shiika repository is cloned.
  During development this is usually `.`.
- `$SHIIKA_WORK`: the directory used to store intermediate files
  (compiled packages, debug logs, etc.).

## Building

```bash
# Build the compiler
cargo build

# Format the code
cargo fmt
```

## Running Shiika programs

```bash
cargo run -- run examples/hello.sk
```

See `examples/*.sk` for more sample programs.

## Testing

```bash
# Build the core package and run every test under tests/sk/
rake test

# Build and run ./a.sk (also dumps debug logs to $SHIIKA_WORK/debug_logs/)
rake async_test
```

### Integration tests

Directory: `tests/sk/`. Each `*.sk` file is compiled and run by `rake test`.
A passing test prints `ok`; a failure prints a message such as `ng: ...` or
exits with a non-zero status.

Run a subset with the `FILTER=` environment variable (substring match on the
path):

```bash
# Run only tests/sk/*block*.sk
FILTER=block rake test
```

Also, `rake examples_test` executes `examples/*.sk` and checks if the output is ass expected.

## Debugging

### Debug logs (`$SHIIKA_WORK/debug_logs/`)

Each MIR lowering pass writes a separate log file, named `01-mirgen.log`
through `07-resolve_env_op.log`.

Notations used in these logs:

- `[*]` means `Asyncness::Async`
- `[+]` means `Asyncness::Sync`
- `[?]` means `Asyncness::Unknown`

### Debugging the parser

Uncomment the body of `debug_log` in `src/parser/base.rs`:

```rust
    /// Print parser debug log (uncomment to enable)
    pub(super) fn debug_log(&self, _msg: &str) {
        //println!("{}{} {}", self.lv_space(), _msg, self.lexer.debug_info());
    }
```
