# Install

Shiika works on Mac, Linux and Windows (with and without WSL2.)
Only 64bit environments are supported.

## Mac, Linux and WSL2

1. Install [Git](https://git-scm.com/)
1. Install [Rust](https://www.rust-lang.org/)
1. Install LLVM 12
  - eg. `brew install llvm@12` on Mac
  - eg. `sudo apt install llvm-12 clang-12` on Ubuntu
- You may need `sudo apt install cmake` on Ubuntu
- (TODO) `apt install libgc-dev` needed on Ubuntu?

You may need to setup some environment variables. For example, on Mac:

```
export PATH="$(brew --prefix)/opt/llvm@12/bin":$PATH
export LDFLAGS="-L$(brew --prefix)/opt/llvm@12/lib"
export CPPFLAGS="-I$(brew --prefix)/opt/llvm@12/include"
```

and on Ubuntu:

```
export LLC=llc-12
export CLANG=clang-12
```

### Compiling core library

You need to compile corelib before running any Shiika programs. 

```
$ git clone https://github.com/shiika-lang/shiika
$ cd shiika
$ cargo build
$ cd lib/skc_rustlib; cargo build; cd ../../
$ cargo run -- build-corelib
```

The `build-corelib` subcommand compiles core classes (builtin/*.sk) into ./builtin/builtin.bc and ./builtin/exports.json. 

### Running a program

```
$ cargo run -- run examples/hello.sk
```

## Windows (without WSL2)

See [setup_windows.md](./setup_windows.md)

## Tips: specifying cargo target folder

Shiika assumes `cargo` generates artifacts into `./target`. You can change this by `SHIIKA_CARGO_TARGET` envvar.

## Tips: `cargo install`

By `cargo install --path .` you can install the compiler as `~/.cargo/bin/shiika`.
Then you can run Shiika programs by `shiika run foo.sk`.

However you may see errors something like

```
$ shiika run main.sk 
Error: ./builtin/exports.json not found
```

because `shiika` looks for corelib in the current directory by default. You can configure this by setting `SHIIKA_ROOT` to point the cloned repository.

```
export SHIIKA_ROOT=/path/to/repo/of/shiika
```
