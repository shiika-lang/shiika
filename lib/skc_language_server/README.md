# skc_language_server

An implementation of LSP(Language Server Protocol) for Shiika.

## State

Just a prototype; has no actual feature now.

Also, this is a bin-crate provides an executable but eventually this
should be a lib-crate that implements `shiika language-server` command.

## File structures

- backend.rs
  - `Backend::initialize` returns server capabilities to the editor.
  - The rest of the methods forwards messages to `crate::server::Server` asynchronously.
- server.rs
  - The body of the language server.

## Acknowledgement

The basic part of this crate is based on https://github.com/dalance/veryl/tree/master/crates/languageserver . Thank you!
