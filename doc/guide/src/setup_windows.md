# Use Shiika on Windows

The easiest way to try Shiika on Windows is to use WSL2.

Using Shiika without WSL2 is not easy, but possible. This document describes how.

## Prerequisites

- 64bit Windows
- Rust
  - https://www.rust-lang.org/tools/install
- Visual Studio >= 2019 (Tested with Professional but Community edition should suffice too)
- CMake (to build LLVM and bdwgc-alloc)
  - Download from https://cmake.org/download/ and install
- Python3 (to build LLVM)
  - https://www.python.org/downloads/windows/

## Build LLVM

First, you need to build LLVM because it seems that

- the `llvm-sys` crate relies on `llvm-config.exe` (no?) and
- the llvm release package does not contain llvm-config.exe.

Steps to build your own llvm:

- Install Python3
- Get LLVM source
  - https://github.com/llvm/llvm-project/releases/tag/llvmorg-12.0.1 llvm-project-12.0.1.src.tar.xz 
  - You may need 7zip to unpack .xz
- Generate llvm.sln with cmake-gui
  - Open cmake-gui
  - Press `Browse Source...` and select `somewhere/llvmorg-12.0.1/llvm`
  - Press `Browse Build...` and select `somewhere/llvmorg-12.0.1/build` (or anywhere you like)
  - Press `Configure`
  - Put `host=x64` to `Optional toolset to use (argument to -T)`
  - Press `Finish` and wait
  - Set `clang;lld` to `LLVM_ENABLE_PROJECTS` (TODO: not needed?)
  - Press `Generate` and wait
- Build LLVM with VS
  - Open build/llvm.sln with Visual Studio
  - Choose `ALL_BUILD` under `CMakePredefinedTargets` in solution explorer
  - Choose `Debug` `x64` and build (`llvm-config.exe` will not be created with `Release`)

## Build shiika.exe

Make sure these are in the PATH.

```
> clang -v
clang version 12.0.1
...
> cmake --version
cmake version 3.42.2
...
> llvm-config --version
12.0.1
```

Then try `cargo build` and see if succeeds.

```
> git clone https://github.com/shiika-lang/shiika
> cd shiika
> set LLVM_SYS_120_PREFIX=c:\somewhere\llvm-project-12.0.1\build\Debug\bin
> cargo build
```