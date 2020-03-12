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
- [x] Local variables
- [ ] `break`
- [ ] String
- [ ] Array
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

### Prerequisits

- Rust
- LLVM (`brew intall llvm@7`)
- bdw-gc 7.6.0 (Currently the path is hardcorded in src/main.rs. PR welcome)
- Ruby (used to generate boiler-plate library definitions)

### Compile

```
$ bundle install
$ rake build
```

### Run tests

```
$ rake test
```

### How to run a Shiika program

```
$ ./build/debug/shiika run examples/fib.sk
```

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
