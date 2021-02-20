# Shiika

Shiika is a statically-typed, Ruby-like programming language.

Ruby has been my "mother tongue" since 2000. What I love about Ruby are:

- Easy to write
  - Method call without parenthesis (eg. `p foo`)
  - Handy syntaxes like `#{}`, modifier `if`, etc.
  - Powerful, small number of core classes (eg. Array also behaves as stack or queue)

On the other hand, static typing has many merits.

- Better performance (it makes optimization easier)
- Easy to refactor (by checking type errors without execution)

Shiika tries to combine these.

Most of the static typing languages, such as C++/Java/Scala/Go/Swift/Kotlin/Rust, etc. are designed for execution speed. However what I want a "lightweight" static typing language.

## Key features

- Ruby-like syntax
- Static type checking
- Everything is an object
- Written in Rust, compiles to LLVM IR

### Design policy

- Easiness over performance
  - Shiika is a glue language. Use C or Rust for performance-critical parts and load it as a library
- Readability matters
  - The name "Shiika" comes from Japanese word "詩歌"(poetry). It should be pleasant to read Shiika programs, not only to write them.
- Easy to learn
  - There may be more than one way to do it, but not too many.
- Scalable
  - Shiika is not only for small programs; Ruby is designed as a "scripting" language but used in production now

### Why not [Crystal](https://crystal-lang.org/)?

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
A.new.fib(34)
```

See `examples/*.sk` for more.

## Documents

- [Lanugage Guide](../doc/guide/src/SUMMARY.md)
- [Lanugage Specification](../doc/spec/src/SUMMARY.md)
- [Hacking Guide](../doc/shg/src/SUMMARY.md)

## Status

Early-alpha but at least capable of solving algorithmic problems like [Advent of Code](https://github.com/yhara/adventofcode)

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
- [x] - Generic methods
- [ ] - Enums
- [ ] - Modules (like Ruby's `module`)
- [ ] - Something like Ruby's `require`
- After v1.0.0
  - Language enhancement
    - Default arguments
    - Keyword arguments passing
    - Pattern matching
    - Exceptions?
  - Built-in library
    - Bignum, etc
  - Standard library?
    - Http, etc?
  - Package system
  - Some meta-programming feature (but not AST macro, sorry lisp fans)

## Hacking

### Prerequisites

- Rust
- LLVM (`brew install llvm@9`)
- bdw-gc (`brew install bdw-gc`)

```sh
export PATH="$(brew --prefix)/opt/llvm@9/bin":$PATH
export LDFLAGS="-L$(brew --prefix)/opt/llvm@9/lib -L$(brew --prefix)/opt/bdw-gc/lib"
export CPPFLAGS="-I$(brew --prefix)/opt/llvm@9/include"
```

### Compile

```
$ cargo build
```

### Run tests

```
$ cargo test
```

Only integration tests (test/sk/*.sk):

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

### How to run a Shiika program

```
$ cargo run -- run examples/hello.sk
```

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
