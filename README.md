a statically-typed programming language.

It looks like Ruby, but has explicit type annotations.
Aims to be Kotlin or Swift in Rubyish style.

The name "Shiika" comes from Japanese word "詩歌"(poetry).
It should be pleasant to read Shiika programs, not only to write them.

## Key features

- Ruby-like syntax
- Compiles to LLVM IRs
- Static type checking
- Consistency: everything is an object
- Is a "scripting" language (prefer easiness over performance; use C or Rust for performance-critical parts and load it as a library)
  - This does not mean Shiika is only for small programs. You know, Ruby is designed as a scripting language but it is considered "production-ready" nowadays

### Why not [Crystal](https://crystal-lang.org/)?

Shiika has lots in common with Crystal. However:

- In Shiika, type annotation of method parameters are mandatory. This helps reading programs written by others
- Shiika has only one class `Int` for integers (cf. `Int8`, `Int16`, `Int32` in Crystal)
- Shiika does not have union types. The type system is more similar to languages such as Rust, Java or Swift (this isn't good or bad; just a difference)

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
- LLVM (`brew install llvm@7`)
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
