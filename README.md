# ![logo](shiika_logo_small.png) Shiika

Shiika is a programming language that makes me most productive.

- Easy to write like Ruby or Python
- Static type checking (Null safety!)
- Object-oriented but has enums and pattern-matching
- Written in Rust, compiles to single binary via LLVM IR

## Concept

Most of the static typing languages, such as C++/Java/Scala/Go/Swift/Kotlin/Rust, etc. are designed for execution speed. However what I want a "lightweight" static typing language to make application faster.

### Design policy

- Easiness over performance
  - Shiika is a glue language. Use Rust (or C, etc.) for performance-critical parts and load it as a library
- Easy to learn
  - There may be more than one way to do it, but not too many.

### Comparison to [Crystal](https://crystal-lang.org/)

Shiika has lots in common with Crystal. However:

- In Shiika, type annotation of method parameters are mandatory. This helps reading programs written by others
- Shiika has only one class `Int` for integers (cf. `Int8`, `Int16`, `Int32` in Crystal)
- Shiika does not have union types. The type system is more similar to languages such as Rust, Java or Swift (this isn't good or bad; just a difference)

## Example

```
class A
  def fib(n: Int) -> Int
    if n < 3
      1
    else
      fib(n-1) + fib(n-2)
    end
  end
end
p A.new.fib(34)
```

See `examples/*.sk` for more.

## Install

→ [install.md](./doc/guide/src/install.md)

## Documents

- [Language Guide](./doc/guide/src/SUMMARY.md)
- [Language Specification](./doc/spec/src/SUMMARY.md)
- [Development Guide](./doc/shg/src/SUMMARY.md)

## Status

Early-alpha; capable of solving algorithmic problems like [Advent of Code](https://github.com/yhara/adventofcode) but a lot more stdlib is needed for practical application.

### Features already implemented

- Classes, Modules, Enums
- Basic Generics
- Basic pattern-matching
- Anonymous function
- Core classes - Object, Array, String, Bool, Int, Float, Dict, Maybe, Class, Metaclass

See [tests/sk/](https://github.com/shiika-lang/shiika/tree/master/tests/sk) and
[examples/](https://github.com/shiika-lang/shiika/tree/master/examples) for more.

### Features not yet implemented

- Something like Ruby's `require`
- Type inference
- More stdlib like `Time`, `File`, etc.

See [Issues](https://github.com/shiika-lang/shiika/issues) for more.

### Roadmap (tentative)

- [x] v0.1.0 - Type system POC
- [x] v0.2.0 - Start writing with Rust
- [x] v0.3.0 - Generics
- [x] v0.4.0 - Anonymous function (lambda)
- [x] v0.5.0 - Virtual methods
- [x] v0.6.0 - Generic methods
- [x] v0.6.0 - Enums, basic pattern matching
- [x] v0.7.0 - Modules (like Ruby's `module`)
- [x] v0.8.0 - Default arguments, keyword arguments
- After v1.0.0
  - Built-in library
    - Bignum, etc
  - Standard library?
    - Http, etc?
  - Package system
  - Some meta-programming feature (but not AST macro, sorry lisp fans)

## Supported platform

- Tested on Mac, Linux(Ubuntu) and Windows
- May not work on 32bit environment

## Hacking

See [install.md](./doc/guide/src/install.md)

### Run tests

```
$ cargo test
```

Only integration tests (test/sk/\*.sk):

```
$ cargo test --test integration_test
```

Specific file under test/sk/ (eg. string.sk):

```
$ FILTER=string cargo test --test integration_test
```

With logging enabled

```
$ RUST_LOG='trace' cargo test
```

## License

MIT

## Contact

https://github.com/shiika-lang/shiika/issues
