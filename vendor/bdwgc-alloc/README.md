# bdwgc-rust

[![GitHub Action](https://img.shields.io/github/actions/workflow/status/bdwgc/bdwgc-rust/test.yaml?branch=main&style=flat-square)](https://github.com/bdwgc/bdwgc-rust/actions)
[![Crate](https://img.shields.io/crates/v/bdwgc-alloc.svg?style=flat-square)](https://crates.io/crates/bdwgc-alloc)
[![License](https://img.shields.io/github/license/bdwgc/bdwgc-rust.svg?style=flat-square)](https://github.com/bdwgc/bdwgc-rust/blob/main/LICENSE)

[`GlobalAlloc`](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html) implementation for [`bdwgc`][bdwgc], the conservative garbage collector.

This crate is for use cases in which developers need to integrate [`bdwgc`][bdwgc] into their programs written in Rust (e.g. writing a runtime library in Rust for their own programming language whose GC is done by [`bdwgc`][bdwgc].)

The raw bindings to the C API of [`bdwgc`][bdwgc] that this crate is built on are available as the `bdwgc-alloc-sys` crate.

## Usage

```rust
use bdwgc_alloc::Allocator;
use std::thread::spawn;

#[global_allocator]
static GLOBAL_ALLOCATOR: Allocator = Allocator;

fn main() {
    unsafe { Allocator::initialize() }

    // Unreachable objects are collected automatically.
    Box::leak(Box::new([0u8; 256]));

    // Threads other than a main thread must be registered to a collector before allocation.
    spawn(|| {
        unsafe { Allocator::register_current_thread() }.unwrap();

        Box::leak(Box::new([0u8; 256]));

        unsafe { Allocator::unregister_current_thread() }
    })
    .join()
    .unwrap();
}
```

By default [`bdwgc`][bdwgc] is built with autotools. To build with cmake, enable the `cmake` feature:

```sh
cargo build --no-default-features --features cmake
```

## License

[MIT](https://github.com/bdwgc/bdwgc-rust/blob/main/LICENSE)

[bdwgc]: https://github.com/bdwgc/bdwgc
