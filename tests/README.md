# Tests

## `tests/sk`

This directory contains various Shiika programs and run by `tests/integration_test.rs`.

### Conventions

.sk in this directory

- must ends with `puts "ok"`
- must not print anything other if succeed
- should print error message if failed

## `tests/erroneous`

This directory contains various Shiika programs which is expected to cause compilation error.
The expected output is stored in `snapshots` directory with the [insta crate](https://insta.rs/docs/).

However this does not mean current error messages are considered perfect; PRs to improve them are welcome. 

Rather than that, erroneous tests are for:

- assuring the type checker detects various type errors
- assuring the compiler does not crash with an erroneous program
- investigating the impact of a modification to the type checker 

### How to add new .sk to `tests/erroneous`

prerequisites: `cargo install cargo-insta`

1. Create .sk
2. `cargo test test_erroneous` (this will create `tests/snapshots/*.snap.new`)
3. `cargo insta review` and accept the snapshot (this will rename `.snap.new` to `.snap`)
4. Commit `.snap` to git

### How to fix CI fail after changing error message

TBA
