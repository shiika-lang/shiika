# Shiika

A statically-typed programming language.

## Example

        class A
          def fib(n: Int) -> Int
            if n < 3
              1
            else
              fib(n-1) + fib(n-2)
            end
          end
        end
        A.new.fib(34)

## Status

Early-alpha

### Implementation (Rust, src/*)

- [x] Implement class method (eg. `Math.pow`)
- [x] Implement .new
- [ ] Local variables
- [ ] Instance variables
- [ ] Blocks
- Constant
  - [x] Toplevel
  - [ ] Namespaced (eg. `A::FOO`)
- [ ] Modules
- [ ] Enums
- ...

#### TODO

- lambda/function(block)
- Module (like Ruby's `Module`)
- Enum
- Constants
- Check all ivars will be initialized (like Swift)
- ...

### Type system (Prototype in Ruby, lib/*)

- [x] Class method, instance method
- [x] Basic generics
- [x] Variable-length arguments
- [x] Array literal
- [x] Inheritance

#### Example

        class Pair<S, T>
          def initialize(@a: S, @b: T)
          end

          def fst -> S; @a; end
          def snd -> T; @b; end
        end
        Pair<Int, Bool>.new(1, true).fst

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
