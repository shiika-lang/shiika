# Tests

Directory: `tests/`

## Unit tests

File: `tests/*_test.rs`

## Integration tests

File: `tests/integration_test.rs`, `tests/sk/*.sk`

These are Shiika-level tests. If the test passes, it should print just `ok`; otherwise, it prints message like `ng foo`.

You can select which .sk to run by `FILTER=` envvar.

```
# Run tests/sk/*block*.sk
$ FILTER=block cargo test --test integration_test -- --nocapture
```

With `--nocapture`, path of the .sk file is printed.

## Doc tests

Some of `src/*.rs` has doc tests.
