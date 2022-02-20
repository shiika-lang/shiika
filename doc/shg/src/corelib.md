# Corelib

Directory: `lib/skc_corelib`, `lib/skc_rustlib`, `builtin`

## corelib

`skc_corelib` defines the core classes like `Object`, `Bool`, `Int` together with its methods.

`skc_rustlib` also defines core methods but written Rust. 

`builtin/*.sk` are Shiika code to define core library.

### Compilation

`builtin/*.sk` and `skc_corelib` are compiled into `builtin/builtin.bc` by `shiika build_corelib`.

`skc_rustlib` is compiled by running `cargo build` (as usual).

`shiika run` executes `clang` to link these with user program.
