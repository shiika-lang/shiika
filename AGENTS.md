# AGENTS.md

This file provides guidance to AI agents when working with code in this repository. 

## Setup

Run `cp dotenvrc .envrc` for direnv.

## Where things are documented

Prefer these docs; only LLM-specific or this-environment-specific notes live here.

- Project overview: `README.md`
- Build / test / run / debug procedures: `DEVELOPMENT.md`
- Compiler internals (such as architecture, pipeline, VTable/WTable, corelib, FFI):
  `doc/shg/src/` (Shiika Hacking Guide)
- Language guide (For newbies): `doc/guide/`
- Language spec (More formal; covers coner cases): `doc/spec/`

## This environment

- `.envrc` contains envvars for this env.
  - `$SHIIKA_ROOT` points to this working copy.
  - `$SHIIKA_WORK`/debug_logs/ : Shiika compiler outputs debug logs here (eg. mir lowering)
  - `$SHIIKA_WORK`/packages/ : Compiled Shiika packages (including .ll)
- `~/.cargo` contains the source of the `inkwell` crate (useful when reading
  LLVM binding APIs).

## Tool use

- Use `jq` for JSON file investigation.
- Use Rust LSP where possible rather than `grep`.

## Git and GitHub rules

- Use `Co-authored-by:` for git commit message.
- Start PR title with `TYPE(AREA): `
  - TYPE
    - fix - fixes a bug
    - feat - adds new feature
    - change - breaking change
    - perf - performance optimization
    - refactor - refactoring
  - AREA
    - lang - the language itself
    - core - packages/core
  - You can omit this prefix if it does not fit in these categories.
