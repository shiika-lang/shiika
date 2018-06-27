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

### Done

- [x] Class method, instance method
- [x] Basic generics
- [x] Variable-length arguments
- [x] Array literal
- [x] Allow omitting `-> Void`

### Todo (short-term)

- String class
- Hash class

### Todo (middle term)

- Subtyping
- lambda/function(block)
- Module (like Ruby's `Module`)
- Constants
- ...

### Todo (long term)

- Generate LLVM IR

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
