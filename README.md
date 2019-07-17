# Shiika

A statically-typed programming language.

## Example

        class Pair<S, T>
          def initialize(@a: S, @b: T)
          end

          def fst -> S; @a; end
          def snd -> T; @b; end
        end
        Pair<Int, Bool>.new(1, true).fst

## Status

Early-alpha

### Implementation (Rust, src/*)

- [x] Parse and emit LLVM IR for minimal case 
- [x] Embed arg name to .ll for better debuggability
- [x] Check type of return value
- [x] Implement class method (eg. `Math.pow`)
- [ ] Implement .new
- [ ] Instance variables
- [ ] Nested class
- [ ] Constant

### Type system (Prototype in Ruby, lib/*)

#### Done

- [x] Class method, instance method
- [x] Basic generics
- [x] Variable-length arguments
- [x] Array literal
- [x] Inheritance

#### Todo

- lambda/function(block)
- Module (like Ruby's `Module`)
- Enum
- Constants
- Check all ivars will be initialized (like Swift)
- ...

## Development

### How to run tests

Prerequisits: Rust, LLVM (`brew intall llvm@7`)

```
$ cargo test
```

### How to compile a Shiika program

Prerequisits: Rust, LLVM, Boehm GC (`brew install bdw-gc`)

1. Edit the program in src/main.rs
2. `cargo run` to generate `a.ll`
3. `llc a.ll` to generate `a.s`
4. `cc -I/usr/local/Cellar/bdw-gc/7.6.0/include/ -L/usr/local/Cellar/bdw-gc/7.6.0/lib/ -lgc -o a.out a.s`
5. `./a.out`

You can also do this by `rake run`, if you have Ruby and Rake installed.

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
