# Corelib

Directory: `src/corelib`, `builtin`

## corelib

`src/corelib` defines the core classes like `Object`, `Bool`, `Int` together with its methods.

`builtin/*.sk` also defines core methods but written in Shiika. These are compiled together with user program.

When adding a core method, you should add it to `builtin` unless it needs some Rust-level feature.
