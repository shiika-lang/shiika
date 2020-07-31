# Shiika is ...

a statically-typed programming language.

It looks like Ruby, but has explicit type annotation.
Aims to be Kotlin or Swift in Rubyish style.

The name "Shiika" comes from Japanese word "詩歌"(poetry).
It should be pleasant to read Shiika programs, not only to write them.

## Key features

- Ruby-like syntax
- Compiles to LLVM IR
- Static type checking
- Consistency: everything is object

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

See `examples/*.sk` for more.

## Status

Early-alpha

### Implementation

- [x] Implement class method (eg. `Math.pow`)
- [x] Implement .new
- [x] Local variables
- [x] String
- [x] Array
- [ ] `break`
- [ ] Instance variables
- [ ] Blocks
- Constant
  - [x] Toplevel
  - [ ] Namespaced (eg. `A::FOO`)
- [ ] Modules
- [ ] Enums
- [ ] Lambda
- ...

## Hacking

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
$ ./build/debug/shiika run examples/hello.sk
```

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
