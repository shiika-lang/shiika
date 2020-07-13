# Tests

Directory: `tests/`

## Unit tests

File: `tests/*_test.rs`

## Integration tests

File: `tests/integration_test.rs`, `tests/sk/*.sk`

These are Shiika-level tests. If the test passes, it should print nothing; otherwise, it prints message like `ng 3` where `3` is a unique number for the test case.

## Doc tests

Some of `src/*.rs` has doc tests.
