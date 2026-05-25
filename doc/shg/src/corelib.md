# Corelib

Directory: `packages/core`

## core package

The `core` package contains the core classes like `Object`, `Bool`, `Int` together with their methods.

- `packages/core/lib/*.sk` are Shiika code defining the core library.
- `packages/core/ext/` contains Rust implementations exposed to Shiika via FFI. The Rust-side method signatures are listed in `packages/core/ext/exports.json5`.

### Compilation

The package is compiled by `shiika build packages/core` and the artifacts go under `$SHIIKA_WORK/packages/core-x.x.x/`.

`shiika run` invokes `clang` to link these with the user program.
