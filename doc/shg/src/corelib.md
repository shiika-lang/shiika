# Corelib

Directory: `src/corelib`, `src/rustlib`, `builtin`

## corelib

`src/corelib` defines the core classes like `Object`, `Bool`, `Int` together with its methods.

`src/rustlib` also defines core methods but written Rust. 

`builtin/*.sk` are Shiika code to define core library.

### Compilation

`builtin/*.sk` and `src/corelib` are compiled into `builtin/builtin.bc` by `shiika build_corelib`.

`src/rustlib` is compiled by running `cargo build` under `src/rustlib`.

`shiika run` executes `clang` to link these with user program.
