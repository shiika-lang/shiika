# New runtime for concurrency

Shiika will have concurrent feature with this new runtime using tokio.

Tracking issue: https://github.com/shiika-lang/shiika/issues/545

## Files

- src/bin/exp_shiika.rs
- lib/skc_async_experiment/
- packages/core

## How to try

1. `cargo run --bin exp_shiika -- build packages/core`
1. `cargo run --bin exp_shiika -- a.sk"`

## Status

Currently the syntax is the same as Shiika (as using lib/shiika_parser)
but some features are not implemented yet. Please check the github issue above.
