# Shiika

is a statically-typed programming language.

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

```crystal
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
```

See `examples/*.sk` for more.

## Status

Early-alpha

### Features already implemented

See [tests/sk/](https://github.com/yhara/shiika/tree/master/tests/sk) and
[examples/](https://github.com/yhara/shiika/tree/master/examples)

### Features not yet implemented

See [Issues](https://github.com/yhara/shiika/issues)

### Roadmap (tentative)

- [x] v0.1.0 - Type system POC
- [x] v0.2.0 - Start writing with Rust
- [x] v0.3.0 - Generics
- [x] v0.4.0 - Anonymous function (lambda)
- [x] v0.5.0 - Virtual methods
- [ ] - Enums
- [ ] - Generic methods
- [ ] - Modules (like Ruby's `module`)
- [ ] - Something like Ruby's `require`
- After v1.0.0
  - Language enhancement
    - Default arguments
    - Keyword arguments passing
    - Pattern matching
    - Exceptions?
  - Built-in library
    - Bignum, Hash, etc
  - Standard library?
    - Http, etc?
  - Package system
  - Some meta-programming feature (but not AST macro, sorry lisp fans)

## Hacking

### Prerequisits

- Rust
- LLVM (`brew install llvm@7`)
- bdw-gc (`brew install bdw-gc`)

### Compile

```
$ cargo build
```

### Run tests

```
$ cargo test
```

### How to run a Shiika program

```
$ cargo run -- run examples/hello.sk
```

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
