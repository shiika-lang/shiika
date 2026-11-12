# Corelib

Directory: `packages/core`

## core package

The `core` package contains the core classes like `Object`, `Bool`, `Int` together with their methods.

- `packages/core/lib/*.sk` are Shiika code defining the core library.
- `packages/core/ext/` contains Rust implementations exposed to Shiika via FFI. Their signatures are declared in `packages/core/lib/*.sk` with `native def` (or `native_async def` for an async Rust function), eg.

  ```sk
  class Int
    # Returns `self` + `other`
    native def +(other: Int) -> Int
  end
  ```

  A `native def` has no body. It can be used in any package which has `rust_libs` in its `package.json5`.

### Compilation

The package is compiled by `shiika build packages/core` and the artifacts go under `$SHIIKA_WORK/packages/core-x.x.x/`.

`shiika run` invokes `clang` to link these with the user program.

## Package layout

- `packages/core`: defines core classes.
- `packages/core/ext`: defines core methods in Rust.
- `packages/core/lib`: defines core methods in Shiika (and declares the Rust ones with `native def`).
- `$SHIIKA_WORK/packages/core-x.x.x/lib/index.bc`: compiled core package (LLVM bitcode).
- `$SHIIKA_WORK/packages/core-x.x.x/lib/exports.json`: interface information of the core package.
