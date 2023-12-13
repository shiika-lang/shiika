# Install

Shiika works on Mac, Linux and Windows (with and without WSL2.)
Only 64bit environments are supported.

## Mac, Linux and WSL2

1. Install [Git](https://git-scm.com/)
1. Install [Rust](https://www.rust-lang.org/). Use latest stable version.
2. Install LLVM 16
  - eg. `brew install llvm@16` on Mac
  - eg. `sudo apt install llvm-16 clang-16` on Ubuntu
    - Try https://apt.llvm.org/ if llvm-16 not available
- You may need `sudo apt install cmake` on Ubuntu
- (TODO) `apt install libgc-dev` needed on Ubuntu?

You may need to setup some environment variables. For example, on Mac:

```
export PATH="$(brew --prefix)/opt/llvm@16/bin":$PATH
export LDFLAGS="-L$(brew --prefix)/opt/llvm@16/lib"
export CPPFLAGS="-I$(brew --prefix)/opt/llvm@16/include"
```

and on Ubuntu:

```
export LLC=llc-16
export CLANG=clang-16
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

The `build-corelib` subcommand compiles core classes (builtin/\*.sk) into ./builtin/builtin.bc and ./builtin/exports.json. 

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

## Troubleshooting

```
error: could not find native static library `Polly`, perhaps an -L flag is missing?

error: could not compile `llvm-sys` due to previous error
```

=> `sudo apt install libpolly-16-dev`

This may happen when you install llvm from https://apt.llvm.org/ .

```
  /Users/yhara/.cargo/registry/src/github.com-1ecc6299db9ec823/bdwgc-alloc-0.6.0/vendor/libatomic_ops/configure: line 4683: syntax error near unexpected token `disable-shared'                                                                        
  /Users/yhara/.cargo/registry/src/github.com-1ecc6299db9ec823/bdwgc-alloc-0.6.0/vendor/libatomic_ops/configure: line 4683: `LT_INIT(disable-shared)'  
```

=> `brew install libtool`
